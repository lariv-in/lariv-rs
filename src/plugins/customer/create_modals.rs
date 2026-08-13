//! Typed [`CreateModal`] / [`PickerModal`] wiring for customer swap keys.

use super::keys::{CustomerCreateModalKey, CustomerSelectModalKey, CustomerSelectTableKey};
use super::routes::{CustomerCreateGetRouteTag, CustomerCreatePostRouteTag};

crate::impl_create_modal!(
    CustomerCreateModalKey,
    CustomerCreateGetRouteTag,
    CustomerCreatePostRouteTag,
    "p_customer.CustomerCreateForm"
);
crate::impl_picker_modal!(CustomerSelectModalKey, CustomerSelectTableKey);

#[cfg(test)]
mod tests {
    use crate::picker::picker_create_button;
    use crate::plugins::customer::keys::CustomerCreateModalKey;

    #[test]
    fn customer_picker_create_button_embeds_target_input() {
        let html = picker_create_button::<CustomerCreateModalKey>(
            "CustomerID",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=CustomerID"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/customers/create/?name=p_customer.CustomerCreateForm&amp;target_input=CustomerID""#
            ),
            "{html}"
        );
        assert!(
            html.contains("name=p_customer.CustomerCreateForm"),
            "{html}"
        );
        assert_eq!(
            html.matches("name=p_customer.CustomerCreateForm").count(),
            1,
            "{html}"
        );
    }
}
