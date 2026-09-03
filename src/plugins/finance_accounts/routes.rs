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

crate::define_plugin_routes! {
    plugin: FinanceAccountsTag;
    routes: [
        get FinanceDefaultRouteTag, "/finance", handlers::accounts::list, fragment(AccountTableKey);
        get AccountCreateGetRouteTag, "/finance/accounts/create", handlers::accounts::create_get, modal;
        post AccountCreatePostRouteTag, "/finance/accounts/create", handlers::accounts::create_post;
        get AccountSelectRouteTag, "/finance/accounts/select", handlers::accounts::select, fk_select(AccountSelectTableKey, AccountSelectModalKey);
        get AccountDetailRouteTag, "/finance/accounts/{id}", handlers::accounts::detail;
        get AccountJournalEntriesRouteTag, "/finance/accounts/{id}/journal-entries", handlers::accounts::journal_entries, fragment(AccountJournalEntriesTableKey);
        get AccountJournalEntryItemsRouteTag, "/finance/accounts/{id}/journal-entry-items", handlers::accounts::journal_entry_items, fragment(AccountJournalEntryItemsTableKey);
        get AccountEditGetRouteTag, "/finance/accounts/{id}/edit", handlers::accounts::edit_get, modal;
        post AccountEditPostRouteTag, "/finance/accounts/{id}/edit", handlers::accounts::edit_post;
        get AccountDeleteGetRouteTag, "/finance/accounts/{id}/delete", handlers::accounts::delete_get, modal;
        post AccountDeletePostRouteTag, "/finance/accounts/{id}/delete", bare handlers::accounts::delete_post, fragment(AccountDeleteModalKey);

        get CurrencyListRouteTag, "/finance/currencies", handlers::currencies::list, fragment(CurrencyTableKey);
        get CurrencyCreateGetRouteTag, "/finance/currencies/create", handlers::currencies::create_get, modal;
        post CurrencyCreatePostRouteTag, "/finance/currencies/create", handlers::currencies::create_post;
        get CurrencySelectRouteTag, "/finance/currencies/select", handlers::currencies::select, fk_select(CurrencySelectTableKey, CurrencySelectModalKey);
        get CurrencyDetailRouteTag, "/finance/currencies/{id}", handlers::currencies::detail;
        get CurrencyEditGetRouteTag, "/finance/currencies/{id}/edit", handlers::currencies::edit_get, modal;
        post CurrencyEditPostRouteTag, "/finance/currencies/{id}/edit", handlers::currencies::edit_post;
        get CurrencyDeleteGetRouteTag, "/finance/currencies/{id}/delete", handlers::currencies::delete_get, modal;
        post CurrencyDeletePostRouteTag, "/finance/currencies/{id}/delete", bare handlers::currencies::delete_post, fragment(CurrencyDeleteModalKey);

        get JournalListRouteTag, "/finance/journals", handlers::journals::list, fragment(JournalTableKey);
        get JournalCreateGetRouteTag, "/finance/journals/create", handlers::journals::create_get, modal;
        post JournalCreatePostRouteTag, "/finance/journals/create", handlers::journals::create_post;
        get JournalSelectRouteTag, "/finance/journals/select", handlers::journals::select, fk_select(JournalSelectTableKey, JournalSelectModalKey);
        get JournalDetailRouteTag, "/finance/journals/{id}", handlers::journals::detail;
        get JournalEditGetRouteTag, "/finance/journals/{id}/edit", handlers::journals::edit_get, modal;
        post JournalEditPostRouteTag, "/finance/journals/{id}/edit", handlers::journals::edit_post;
        get JournalDeleteGetRouteTag, "/finance/journals/{id}/delete", handlers::journals::delete_get, modal;
        post JournalDeletePostRouteTag, "/finance/journals/{id}/delete", bare handlers::journals::delete_post, fragment(JournalDeleteModalKey);

        get JournalEntryCreateGetRouteTag, "/finance/journals/{journal_id}/entries/create", handlers::journal_entries::create_get, param journal_id: i64, modal;
        post JournalEntryCreatePostRouteTag, "/finance/journals/{journal_id}/entries/create", handlers::journal_entries::create_post, param journal_id: i64;
        get JournalEntryDetailRouteTag, "/finance/journal-entries/{id}", handlers::journal_entries::detail;
        get JournalEntryDeleteGetRouteTag, "/finance/journal-entries/{id}/delete", handlers::journal_entries::delete_get, modal;
        post JournalEntryDeletePostRouteTag, "/finance/journal-entries/{id}/delete", bare handlers::journal_entries::delete_post, fragment(JournalEntryDeleteModalKey);
        get JournalEntrySelectRouteTag, "/finance/journal-entries/select", handlers::journal_entries::select, fk_select(JournalEntrySelectTableKey, JournalEntrySelectModalKey);
        get SourceDocSelectRouteTag, "/finance/source-docs/select", handlers::source_docs::select, fk_select(SourceDocSelectTableKey, SourceDocSelectModalKey);

        get AccountingPreferencesRouteTag, "/finance/preferences", handlers::preferences::get;
        post AccountingPreferencesPostRouteTag, "/finance/preferences", handlers::preferences::post;
    ]
}
