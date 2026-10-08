use super::{
    handlers,
    keys::{
        AccountDeleteModalKey, AccountJournalEntriesTableKey, AccountJournalEntryItemsTableKey,
        AccountSelectModalKey, AccountSelectTableKey, AccountTableKey, CurrencyDeleteModalKey,
        CurrencySelectModalKey, CurrencySelectTableKey, CurrencyTableKey, JournalDeleteModalKey,
        JournalEntryDeleteModalKey, JournalEntrySelectModalKey, JournalEntrySelectTableKey,
        JournalSelectModalKey, JournalSelectTableKey, JournalTableKey, SourceDocSelectModalKey,
        SourceDocSelectTableKey,
    },
};

pub struct FinanceAccountsView;

pub struct FinanceAccountsMutate;

/// Saving `/finance/preferences`. Empty allowlist: superuser only.
pub struct FinanceAccountsPreferencesMutate;

lariv_core::define_plugin_routes! {
    plugin: FinanceAccountsTag;
    prefix: "/dashboard";
    routes: [
        get FinanceDefaultRouteTag, "/finance", handlers::accounts::list, fragment(AccountTableKey), authorize(FinanceAccountsView, []);
        get AccountCreateGetRouteTag, "/finance/accounts/create", handlers::accounts::create_get, modal, authorize(FinanceAccountsMutate, []);
        post AccountCreatePostRouteTag, "/finance/accounts/create", handlers::accounts::create_post, authorize(FinanceAccountsMutate, []);
        get AccountSelectRouteTag, "/finance/accounts/select", handlers::accounts::select, fk_select(AccountSelectTableKey, AccountSelectModalKey), authorize(FinanceAccountsView, []);
        get AccountDetailRouteTag, "/finance/accounts/{id}", handlers::accounts::detail, authorize(FinanceAccountsView, []);
        get AccountJournalEntriesRouteTag, "/finance/accounts/{id}/journal-entries", handlers::accounts::journal_entries, fragment(AccountJournalEntriesTableKey), authorize(FinanceAccountsView, []);
        get AccountJournalEntryItemsRouteTag, "/finance/accounts/{id}/journal-entry-items", handlers::accounts::journal_entry_items, fragment(AccountJournalEntryItemsTableKey), authorize(FinanceAccountsView, []);
        get AccountEditGetRouteTag, "/finance/accounts/{id}/edit", handlers::accounts::edit_get, modal, authorize(FinanceAccountsMutate, []);
        post AccountEditPostRouteTag, "/finance/accounts/{id}/edit", handlers::accounts::edit_post, authorize(FinanceAccountsMutate, []);
        get AccountDeleteGetRouteTag, "/finance/accounts/{id}/delete", handlers::accounts::delete_get, modal, authorize(FinanceAccountsMutate, []);
        post AccountDeletePostRouteTag, "/finance/accounts/{id}/delete", bare handlers::accounts::delete_post, fragment(AccountDeleteModalKey), authorize(FinanceAccountsMutate, []);

        get CurrencyListRouteTag, "/finance/currencies", handlers::currencies::list, fragment(CurrencyTableKey), authorize(FinanceAccountsView, []);
        get CurrencyCreateGetRouteTag, "/finance/currencies/create", handlers::currencies::create_get, modal, authorize(FinanceAccountsMutate, []);
        post CurrencyCreatePostRouteTag, "/finance/currencies/create", handlers::currencies::create_post, authorize(FinanceAccountsMutate, []);
        get CurrencySelectRouteTag, "/finance/currencies/select", handlers::currencies::select, fk_select(CurrencySelectTableKey, CurrencySelectModalKey), authorize(FinanceAccountsView, []);
        get CurrencyDetailRouteTag, "/finance/currencies/{id}", handlers::currencies::detail, authorize(FinanceAccountsView, []);
        get CurrencyEditGetRouteTag, "/finance/currencies/{id}/edit", handlers::currencies::edit_get, modal, authorize(FinanceAccountsMutate, []);
        post CurrencyEditPostRouteTag, "/finance/currencies/{id}/edit", handlers::currencies::edit_post, authorize(FinanceAccountsMutate, []);
        get CurrencyDeleteGetRouteTag, "/finance/currencies/{id}/delete", handlers::currencies::delete_get, modal, authorize(FinanceAccountsMutate, []);
        post CurrencyDeletePostRouteTag, "/finance/currencies/{id}/delete", bare handlers::currencies::delete_post, fragment(CurrencyDeleteModalKey), authorize(FinanceAccountsMutate, []);

        get JournalListRouteTag, "/finance/journals", handlers::journals::list, fragment(JournalTableKey), authorize(FinanceAccountsView, []);
        get JournalCreateGetRouteTag, "/finance/journals/create", handlers::journals::create_get, modal, authorize(FinanceAccountsMutate, []);
        post JournalCreatePostRouteTag, "/finance/journals/create", handlers::journals::create_post, authorize(FinanceAccountsMutate, []);
        get JournalGenerateGetRouteTag, "/finance/journals/generate", handlers::journals::generate_get, modal, authorize(FinanceAccountsMutate, []);
        post JournalGeneratePostRouteTag, "/finance/journals/generate", handlers::journals::generate_post, authorize(FinanceAccountsMutate, []);
        get JournalSelectRouteTag, "/finance/journals/select", handlers::journals::select, fk_select(JournalSelectTableKey, JournalSelectModalKey), authorize(FinanceAccountsView, []);
        get JournalDetailRouteTag, "/finance/journals/{id}", handlers::journals::detail, authorize(FinanceAccountsView, []);
        get JournalEditGetRouteTag, "/finance/journals/{id}/edit", handlers::journals::edit_get, modal, authorize(FinanceAccountsMutate, []);
        post JournalEditPostRouteTag, "/finance/journals/{id}/edit", handlers::journals::edit_post, authorize(FinanceAccountsMutate, []);
        get JournalDeleteGetRouteTag, "/finance/journals/{id}/delete", handlers::journals::delete_get, modal, authorize(FinanceAccountsMutate, []);
        post JournalDeletePostRouteTag, "/finance/journals/{id}/delete", bare handlers::journals::delete_post, fragment(JournalDeleteModalKey), authorize(FinanceAccountsMutate, []);

        get JournalEntryCreateGetRouteTag, "/finance/journals/{journal_id}/entries/create", handlers::journal_entries::create_get, param journal_id: i64, modal, authorize(FinanceAccountsMutate, []);
        post JournalEntryCreatePostRouteTag, "/finance/journals/{journal_id}/entries/create", handlers::journal_entries::create_post, param journal_id: i64, authorize(FinanceAccountsMutate, []);
        get JournalEntryDetailRouteTag, "/finance/journal-entries/{id}", handlers::journal_entries::detail, authorize(FinanceAccountsView, []);
        get JournalEntryDeleteGetRouteTag, "/finance/journal-entries/{id}/delete", handlers::journal_entries::delete_get, modal, authorize(FinanceAccountsMutate, []);
        post JournalEntryDeletePostRouteTag, "/finance/journal-entries/{id}/delete", bare handlers::journal_entries::delete_post, fragment(JournalEntryDeleteModalKey), authorize(FinanceAccountsMutate, []);
        get JournalEntrySelectRouteTag, "/finance/journal-entries/select", handlers::journal_entries::select, fk_select(JournalEntrySelectTableKey, JournalEntrySelectModalKey), authorize(FinanceAccountsView, []);
        get SourceDocSelectRouteTag, "/finance/source-docs/select", handlers::source_docs::select, fk_select(SourceDocSelectTableKey, SourceDocSelectModalKey), authorize(FinanceAccountsView, []);

        get AccountingPreferencesRouteTag, "/finance/preferences", handlers::preferences::get, authorize(FinanceAccountsView, []);
        post AccountingPreferencesPostRouteTag, "/finance/preferences", handlers::preferences::post, authorize(FinanceAccountsPreferencesMutate, []);
    ]
}
