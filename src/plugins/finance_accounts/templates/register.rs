use crate::{
    components::{SlotCapability, SlotRegistrar},
    http::ProvideRequestCaps,
    template::{TemplateCapability, TemplateOf, TemplateRegistrar},
};


use super::{
    accounts::{
        AccountCreateModalPage, AccountDetailPage, AccountEditModalPage, AccountJournalEntriesPage,
        AccountJournalEntryItemsPage, AccountListPage, AccountSelectPage,
    },
    currencies::{
        CurrencyCreateModalPage, CurrencyDetailPage, CurrencyEditModalPage, CurrencyListPage,
        CurrencySelectPage,
    },
    journals::{
        JournalCreateModalPage, JournalDetailPage, JournalEditModalPage, JournalEntryCreateModalPage,
        JournalEntryDeletePage, JournalEntryDetailPage, JournalEntrySelectPage, JournalListPage,
        JournalSelectPage,
    },
    preferences::AccountingPreferencesPage,
    source_docs::SourceDocSelectPage,
};

crate::define_register_items! {
    plugin: FinanceAccountsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        AccountListIdx: AccountListPageTag => AccountListPage,
        AccountDetailIdx: AccountDetailPageTag => AccountDetailPage,
        AccountJournalEntriesIdx: AccountJournalEntriesPageTag => AccountJournalEntriesPage,
        AccountJournalEntryItemsIdx: AccountJournalEntryItemsPageTag => AccountJournalEntryItemsPage,
        AccountEditModalIdx: AccountEditModalPageTag => AccountEditModalPage,
        AccountCreateModalIdx: AccountCreateModalPageTag => AccountCreateModalPage,
        AccountSelectIdx: AccountSelectPageTag => AccountSelectPage,
        CurrencyListIdx: CurrencyListPageTag => CurrencyListPage,
        CurrencyDetailIdx: CurrencyDetailPageTag => CurrencyDetailPage,
        CurrencyEditModalIdx: CurrencyEditModalPageTag => CurrencyEditModalPage,
        CurrencyCreateModalIdx: CurrencyCreateModalPageTag => CurrencyCreateModalPage,
        CurrencySelectIdx: CurrencySelectPageTag => CurrencySelectPage,
        JournalListIdx: JournalListPageTag => JournalListPage,
        JournalDetailIdx: JournalDetailPageTag => JournalDetailPage,
        JournalEditModalIdx: JournalEditModalPageTag => JournalEditModalPage,
        JournalCreateModalIdx: JournalCreateModalPageTag => JournalCreateModalPage,
        JournalSelectIdx: JournalSelectPageTag => JournalSelectPage,
        JournalEntryCreateModalIdx: JournalEntryCreateModalPageTag => JournalEntryCreateModalPage,
        JournalEntryDetailIdx: JournalEntryDetailPageTag => JournalEntryDetailPage,
        JournalEntryDeleteIdx: JournalEntryDeletePageTag => JournalEntryDeletePage,
        JournalEntrySelectIdx: JournalEntrySelectPageTag => JournalEntrySelectPage,
        SourceDocSelectIdx: SourceDocSelectPageTag => SourceDocSelectPage,
        AccountingPreferencesIdx: AccountingPreferencesPageTag => AccountingPreferencesPage,
    ]
}

crate::define_register_items! {
    plugin: FinanceAccountsTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
