# ustring
Fast, FFI-friendly string interning. A UString is a lightweight handle
representing an entry in a global string cache, allowing for: 
* Extremely fast string comparisons - it's just a pointer comparison.
* Amortized storage -  only one copy of the string is held in memory, and 
getting access to it is just a pointer indirection.
* Fast hashing - the precomputed hash is stored with the string
* Fast FFI - the string is stored with a terminating null byte so can be 
passed to C directly without doing the CString dance.

The downside is no strings are ever freed, so if you're creating lots and 
lots of strings, you might run out of memory. On the other hand, War and Peace
is only 3MB, so it's probably fine.

This crate is directly inspired by [OpenImageIO's ustring](https://github.com/OpenImageIO/oiio/blob/master/src/include/OpenImageIO/ustring.h)
but it is NOT binary-compatible (yet). The underlying hash map implementation
is directy ported from OIIO (but without the binning).

```rust
use ustring::{UString, u};
let h1 = u!("hello");
let h2 = u!("hello");
assert_eq!(h1, h2); //< just a pointer comparison
```

# Testing
Note that tests must be run with RUST_TEST_THREADS=1 or some tests will fail due
to concurrent tests filling the cache.

# NOTICE
This crate is pre-alpha. It has been tested (barely) on x86-64. Whatever
your architecture, there's probably undefined behaviour lurking in here, so
be warned. It also requires nightly.

## Why?
It is common in certain types of applications to use strings as identifiers,
but not really do any processing with them. 
To paraphrase from OIIO's ustring documentation - 
Compared to standard strings, ustrings have several advantages:

- Each individual ustring is very small -- in fact, we guarantee that
a ustring is the same size and memory layout as an ordinary *u8.
- Storage is frugal, since there is only one allocated copy of each
unique character sequence, throughout the lifetime of the program.
- Assignment from one ustring to another is just copy of the pointer;
no allocation, no character copying, no reference counting.
- Equality testing (do the strings contain the same characters) is
a single operation, the comparison of the pointer.
- Memory allocation only occurs when a new ustring is constructed from
raw characters the FIRST time -- subsequent constructions of the
same string just finds it in the canonial string set, but doesn't
need to allocate new storage.  Destruction of a ustring is trivial,
there is no de-allocation because the canonical version stays in
the set.  Also, therefore, no user code mistake can lead to
memory leaks.
- Creating a new UString is faster than String::from()

But there are some problems, too.  Canonical strings are never freed
from the table.  So in some sense all the strings "leak", but they
only leak one copy for each unique string that the program ever comes
across.

On the whole, ustrings are a really great string representation
- if you tend to have (relatively) few unique strings, but many
copies of those strings;
- if the creation of strings from raw characters is relatively
rare compared to copying or comparing to existing strings;
- if you tend to make the same strings over and over again, and
if it's relatively rare that a single unique character sequence
is used only once in the entire lifetime of the program;
- if your most common string operations are assignment and equality
testing and you want them to be as fast as possible;
- if you are doing relatively little character-by-character assembly
of strings, string concatenation, or other "string manipulation"
(other than equality testing).

ustrings are not so hot
- if your program tends to have very few copies of each character
sequence over the entire lifetime of the program;
- if your program tends to generate a huge variety of unique
strings over its lifetime, each of which is used only a short
time and then discarded, never to be needed again;
- if you don't need to do a lot of string assignment or equality
testing, but lots of more complex string manipulation.

## Safety and Compatibility
This crate has been tested (a little) on x86_64 ONLY. It might well do
horrible, horrible things on other architectures.

## Licence
Copyright 2019 Anders Langlands

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:
1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
