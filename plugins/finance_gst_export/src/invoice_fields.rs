//! GST return fields shown on an invoice, matching the offline tool's tables.

use maud::{Markup, html};

use crate::details::GstFormValues;

pub fn render(values: &GstFormValues, bulk: bool) -> Markup {
    let section = values.gst_section.as_str();
    let inv_typ = values.gst_inv_typ.as_str();
    let amends =
        !bulk && (!values.amendment_of_number.is_empty() || !values.amendment_of_date.is_empty());
    let amends_js = if amends { "true" } else { "false" };
    let state = format!(
        "{{ gstSection: '{}', gstInvTyp: '{}', gstAmends: {amends_js} }}",
        js_token(section),
        js_token(inv_typ),
    );
    html! {
        div x-data=(state) {
            h3 class="font-semibold mt-2" { "GST return" }
            label class="form-control w-full" {
                span class="label-text font-bold" { "GST section" }
                select class="select select-bordered w-full" name="gst_section" x-model="gstSection" {
                    @if bulk {
                        option value="" selected[section.is_empty()] { "None" }
                    }
                    @for (value, label) in SECTIONS {
                        option value=(value) selected[*value == section] { (label) }
                    }
                }
            }
            fieldset class="border-0 p-0 m-0 min-w-0" x-show="gstSection === 'regular' || gstSection === 'ecom_b2b' || gstSection === 'ecom_urp2b'" x-bind:disabled="!(gstSection === 'regular' || gstSection === 'ecom_b2b' || gstSection === 'ecom_urp2b')" {
                label class="form-control w-full" {
                    span class="label-text font-bold" { "GST invoice type" }
                    select class="select select-bordered w-full" name="gst_inv_typ" x-model="gstInvTyp" {
                        @if bulk {
                            option value="" selected[inv_typ.is_empty()] { "None" }
                        }
                        @for (value, label) in INV_TYPES {
                            option value=(value) selected[*value == inv_typ] { (label) }
                        }
                    }
                }
            }
            fieldset class="border-0 p-0 m-0 min-w-0" x-show="gstSection === 'regular'" x-bind:disabled="gstSection !== 'regular'" {
                fieldset class="border-0 p-0 m-0 min-w-0" x-bind:disabled="gstInvTyp === 'DE'" {
                    label class="label cursor-pointer justify-start gap-2" {
                        input type="checkbox" class="checkbox" name="gst_reverse_charge" value="true" x-bind:checked=(values.reverse_charge == "true" && !bulk) {}
                        span class="label-text font-bold" { "Reverse charge" }
                    }
                }
            }
            fieldset class="border-0 p-0 m-0 min-w-0" x-show="gstSection === 'export'" x-bind:disabled="gstSection !== 'export'" {
                label class="form-control w-full" {
                    span class="label-text font-bold" { "Export type" }
                    select class="select select-bordered w-full" name="gst_exp_typ" {
                        option value="" selected[values.exp_typ.is_empty()] { "None" }
                        option value="WPAY" selected[values.exp_typ == "WPAY"] { "With payment" }
                        option value="WOPAY" selected[values.exp_typ == "WOPAY"] { "Without payment" }
                    }
                }
                (text_field("gst_shipping_bill_number", "Shipping bill number", if bulk { "" } else { values.shipping_bill_number.as_str() }))
                (text_field("gst_shipping_bill_date", "Shipping bill date", if bulk { "" } else { values.shipping_bill_date.as_str() }))
                (text_field("gst_port_code", "Port code", if bulk { "" } else { values.port_code.as_str() }))
            }
            fieldset class="border-0 p-0 m-0 min-w-0" x-show="gstSection === 'ecom_b2b' || gstSection === 'ecom_b2c' || gstSection === 'ecom_urp2b' || gstSection === 'ecom_urp2c'" x-bind:disabled="!(gstSection === 'ecom_b2b' || gstSection === 'ecom_b2c' || gstSection === 'ecom_urp2b' || gstSection === 'ecom_urp2c')" {
                (text_field("gst_supplier_gstin", "E-commerce supplier GSTIN", if bulk { "" } else { values.supplier_gstin.as_str() }))
            }
            fieldset class="border-0 p-0 m-0 min-w-0" x-show="gstSection === 'supeco_clttx' || gstSection === 'supeco_paytx'" x-bind:disabled="!(gstSection === 'supeco_clttx' || gstSection === 'supeco_paytx')" {
                (text_field("gst_etin", "E-commerce operator GSTIN", if bulk { "" } else { values.etin.as_str() }))
            }
            label class="label cursor-pointer justify-start gap-2" {
                input type="checkbox" class="checkbox" name="gst_amends" value="true" x-model="gstAmends" {}
                span class="label-text font-bold" { "This amends an earlier invoice" }
            }
            fieldset class="border-0 p-0 m-0 min-w-0" x-show="gstAmends" x-bind:disabled="!gstAmends" {
                (text_field("gst_amendment_of_number", "Original invoice number", if bulk { "" } else { values.amendment_of_number.as_str() }))
                (text_field("gst_amendment_of_date", "Original invoice date", if bulk { "" } else { values.amendment_of_date.as_str() }))
            }
        }
    }
}

pub fn detail(summary: &str) -> Markup {
    html! {
        label class="form-control" {
            span class="label-text font-bold" { "GST" }
            span { (summary) }
        }
    }
}

fn text_field(name: &str, label: &str, value: &str) -> Markup {
    html! {
        label class="form-control w-full" {
            span class="label-text font-bold" { (label) }
            input type="text" class="input input-bordered w-full" name=(name) value=(value) {}
        }
    }
}

fn js_token(value: &str) -> &'static str {
    match value {
        "regular" => "regular",
        "export" => "export",
        "ecom_b2b" => "ecom_b2b",
        "ecom_b2c" => "ecom_b2c",
        "ecom_urp2b" => "ecom_urp2b",
        "ecom_urp2c" => "ecom_urp2c",
        "supeco_clttx" => "supeco_clttx",
        "supeco_paytx" => "supeco_paytx",
        "R" => "R",
        "SEWP" => "SEWP",
        "SEWOP" => "SEWOP",
        "DE" => "DE",
        "CBW" => "CBW",
        _ => "",
    }
}

const SECTIONS: &[(&str, &str)] = &[
    ("regular", "Regular"),
    ("export", "Export"),
    ("ecom_b2b", "E-commerce B2B"),
    ("ecom_b2c", "E-commerce B2C"),
    ("ecom_urp2b", "E-commerce unregistered to registered"),
    ("ecom_urp2c", "E-commerce unregistered to consumer"),
    ("supeco_clttx", "ECO supplies, tax collected"),
    ("supeco_paytx", "ECO supplies, tax paid"),
];

const INV_TYPES: &[(&str, &str)] = &[
    ("R", "Regular"),
    ("SEWP", "SEZ with payment"),
    ("SEWOP", "SEZ without payment"),
    ("DE", "Deemed export"),
    ("CBW", "Intra-state supplies attracting IGST"),
];
