//! `tests/ui/compat.slint` is also compiled by the official Slint compiler
//! (`tests/slint-compat`). This checks that slint-dom accepts it and generates
//! the same Rust API for every property access qualifier.

slint_dom::include_ui!("tests/ui/compat.slint");

#[test]
fn qualified_properties_have_the_plain_property_api() {
    let _: fn(&Compat) -> String = Compat::status;
    let _: fn(&Compat, String) = Compat::set_status;
    let _: fn(&Compat) -> slint_dom::Property<String> = Compat::status_property;
    let _: fn(&Compat) -> bool = Compat::active;
    let _: fn(&Compat, bool) = Compat::set_active;
    let _: fn(&Compat) -> i32 = Compat::count;
    let _: fn(&Compat, i32) = Compat::set_count;
    let _: fn(&Compat) -> f64 = Compat::level;
    let _: fn(&Compat, f64) = Compat::set_level;
    let _: fn(&Compat) -> &slint_dom::__private::Element = Compat::start_button;
}
