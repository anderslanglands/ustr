//! `Ustr` must reflect as an opaque *string* scalar, never as a struct holding
//! a raw pointer.
//!
//! Deriving `Facet` on `Ustr` used to expose the `char_ptr` field, so a
//! deserializer would write an arbitrary integer straight into the pointer and
//! hand back a handle that dereferences to nothing (observed downstream as
//! `misaligned pointer dereference: address must be a multiple of 0x8 but is
//! 0x200000063`). These tests drive the exact vtable entry points that facet's
//! format crates use, and assert the deserialized handle came out of the global
//! string cache.

#![cfg(feature = "facet")]

use facet::{
    Def, Facet, PtrConst, PtrUninit, Shape, TryFromOutcome, Type, UserType,
};
use std::fmt;
use std::mem::MaybeUninit;
use ustr::{Ustr, ustr};

/// Renders a value through its shape's `display` vtable entry, which is what a
/// text format writes out.
struct DisplayThroughShape {
    shape: &'static Shape,
    value: Ustr,
}

impl fmt::Display for DisplayThroughShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pointer = PtrConst::new_sized(&self.value as *const Ustr);
        unsafe { self.shape.call_display(pointer, f) }
            .expect("shape has no display fn")
    }
}

/// Builds a value through its shape's `parse` vtable entry, which is what a
/// text format uses to deserialize.
fn parse_through_shape(shape: &'static Shape, text: &str) -> Ustr {
    let mut slot = MaybeUninit::<Ustr>::uninit();
    let target = PtrUninit::new_sized(slot.as_mut_ptr());

    unsafe { shape.call_parse(text, target) }
        .expect("shape has no parse fn")
        .expect("parse failed");

    unsafe { slot.assume_init() }
}

fn try_from_through_shape(
    shape: &'static Shape,
    source_shape: &'static Shape,
    source: PtrConst,
) -> Ustr {
    let mut slot = MaybeUninit::<Ustr>::uninit();
    let target = PtrUninit::new_sized(slot.as_mut_ptr());

    let outcome = unsafe { shape.call_try_from(source_shape, source, target) }
        .expect("shape has no try_from fn");
    assert!(
        matches!(outcome, TryFromOutcome::Converted),
        "try_from did not convert"
    );

    unsafe { slot.assume_init() }
}

/// The raw address `Ustr` hands out. Two `Ustr`s for the same text share it iff
/// they came from the same cache entry.
fn interned_address(value: Ustr) -> *const std::ffi::c_char {
    value.as_char_ptr()
}

#[test]
fn shape_is_an_opaque_scalar_not_a_pointer_struct() {
    assert!(
        matches!(Ustr::SHAPE.def, Def::Scalar),
        "Ustr must be a scalar, got {:?}",
        Ustr::SHAPE.def
    );
    assert!(
        matches!(Ustr::SHAPE.ty, Type::User(UserType::Opaque)),
        "Ustr must be opaque; anything else exposes char_ptr to deserializers"
    );

    // The regression itself: no reflected field may carry the raw pointer.
    if let Type::User(UserType::Struct(struct_type)) = Ustr::SHAPE.ty {
        panic!(
            "Ustr reflects {} field(s); a deserializer would write into char_ptr",
            struct_type.fields.len()
        );
    }
}

#[test]
fn display_writes_the_string_not_the_pointer() {
    let value = ustr("hello, cache");

    let rendered = DisplayThroughShape {
        shape: Ustr::SHAPE,
        value,
    }
    .to_string();

    assert_eq!(rendered, "hello, cache");
}

#[test]
fn parse_interns_through_the_global_cache() {
    let expected = ustr("round trip");

    let parsed = parse_through_shape(Ustr::SHAPE, "round trip");

    assert_eq!(parsed.as_str(), "round trip");
    assert_eq!(
        interned_address(parsed),
        interned_address(expected),
        "parse fabricated a handle instead of interning"
    );
    assert_eq!(parsed, expected);
}

#[test]
fn display_then_parse_round_trips_a_bare_ustr() {
    let original = ustr("bare round trip");

    let serialized = DisplayThroughShape {
        shape: Ustr::SHAPE,
        value: original,
    }
    .to_string();
    let deserialized = parse_through_shape(Ustr::SHAPE, &serialized);

    assert_eq!(deserialized.as_str(), original.as_str());
    assert_eq!(interned_address(deserialized), interned_address(original));
}

#[test]
fn try_from_str_interns() {
    let expected = ustr("from a str");
    let source: &str = "from a str";

    let converted = try_from_through_shape(
        Ustr::SHAPE,
        <&str as Facet>::SHAPE,
        PtrConst::new_sized(&source as *const &str),
    );

    assert_eq!(interned_address(converted), interned_address(expected));
}

#[test]
fn try_from_string_interns() {
    let expected = ustr("from a String");
    let source = String::from("from a String");

    let converted = try_from_through_shape(
        Ustr::SHAPE,
        <String as Facet>::SHAPE,
        PtrConst::new_sized(&source as *const String),
    );
    // `try_from` consumed the String.
    std::mem::forget(source);

    assert_eq!(interned_address(converted), interned_address(expected));
}

/// A `Ustr` reached through a struct field — the shape a real config type has.
#[derive(Facet)]
struct Settings {
    name: Ustr,
}

#[test]
fn ustr_field_of_a_struct_round_trips_through_the_cache() {
    let Type::User(UserType::Struct(struct_type)) = Settings::SHAPE.ty else {
        panic!("Settings should reflect as a struct");
    };

    let field = struct_type
        .fields
        .iter()
        .find(|field| field.name == "name")
        .expect("missing `name` field");
    let field_shape = field.shape.get();

    assert_eq!(field_shape.id, Ustr::SHAPE.id);
    assert!(
        matches!(field_shape.def, Def::Scalar),
        "the Ustr field must deserialize as a scalar string"
    );

    let settings = Settings {
        name: ustr("configured name"),
    };

    let serialized = DisplayThroughShape {
        shape: field_shape,
        value: settings.name,
    }
    .to_string();
    assert_eq!(serialized, "configured name");

    let deserialized = parse_through_shape(field_shape, &serialized);
    assert_eq!(deserialized.as_str(), "configured name");
    assert_eq!(
        interned_address(deserialized),
        interned_address(ustr("configured name")),
        "the struct field's Ustr was fabricated, not interned"
    );
}
