//! Request form structs for filesystem admin.

use maud::Markup;

use lariv_core::components::{HtmlAttrs, InputFile, input_file};
use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, Upload, html_form,
    widgets::{Checkbox, CodeEditor, File, ForeignKey, Kind, Role, Section, Text},
};
use crate::routes::VNodeSelectRouteTag;
use lariv_plugin_users::routes::UsersSelectRouteTag;

// Keeps widget types in scope for `widget = …` (macro matches the path; not named in expansion).
const _: fn() = || {
    let _: Kind = Kind;
    let _: FilePrefillName = FilePrefillName;
    let _: CodeEditor = CodeEditor;
    let _: Checkbox = Checkbox;
    let _: Section = Section;
    let _: UsersSelectRouteTag = UsersSelectRouteTag;
};

/// File input that copies the chosen filename into the sibling `Name` field.
pub struct FilePrefillName;

impl FormWidget for FilePrefillName {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_file(InputFile {
            label: field.label,
            name: field.name,
            required: field.required,
            multiple: field.spec.multiple,
            accept: field.spec.accept.unwrap_or(""),
            attrs: HtmlAttrs::new().set(
                "onchange",
                "const n=this.form&&this.form.querySelector('[name=Name]');if(n&&this.files&&this.files[0])n.value=this.files[0].name",
            ),
            ..Default::default()
        })
    }
}

#[html_form(default)]
pub struct MoveForm {
    #[form(
        label = "Destination Folder",
        widget = ForeignKey,
        swap_key = "fk-vnode-destination",
        display = "destination",
        placeholder = "Filesystem root"
    )]
    pub destination_id: i64,
}

#[html_form(default)]
pub enum VNodeKind {
    #[form(label = "Directory")]
    Directory,

    #[form(label = "File")]
    File {
        #[form(label = "File", widget = FilePrefillName, required)]
        file: Upload,
    },
}

#[html_form(default)]
pub struct VNodeForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(widget = Kind)] // Kind: radio discriminant + Alpine variant fields
    pub kind: VNodeKind,

    #[form(
        label = "Parent Folder",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "fk-vnode-parent",
        display = "parent",
        when = "create_mode",
        placeholder = "Filesystem root"
    )]
    pub parent_id: Option<i64>,
}

/// Edit form: name always; optional file replace when editing a file node.
#[html_form(default)]
pub struct VNodeEditForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "File", widget = File, when = "show_file")]
    pub file: Option<Upload>,
}

/// Detail-page text editor: replace a file VNode's contents.
#[html_form]
pub struct VNodeContentForm {
    #[form(label = "Contents", widget = CodeEditor, rows = 20)]
    pub content: String,
}

#[html_form(default)]
pub struct VNodeMultiUploadForm {
    #[form(
        label = "Destination Folder",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "fk-vnode-parent",
        display = "parent",
        placeholder = "Filesystem root"
    )]
    pub parent_id: Option<i64>,

    #[form(label = "Files", widget = File, multiple, required)]
    pub files: Vec<Upload>,
}

#[html_form(default)]
pub struct VNodeZipUploadForm {
    #[form(
        label = "Destination Folder",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "fk-vnode-parent",
        display = "parent",
        placeholder = "Filesystem root"
    )]
    pub parent_id: Option<i64>,

    #[form(label = "Zip File", widget = File, accept = ".zip", required)]
    pub zip_file: Upload,
}

/// Owner, role, and access for one item. The inside-folder checkbox is directories only.
#[html_form(default)]
pub struct VNodePermissionsForm {
    #[form(
        label = "Owner",
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "fk-vnode-owner",
        display = "owner",
        placeholder = "No owner"
    )]
    pub owner_id: Option<i64>,

    #[form(
        label = "Role",
        widget = Role,
        placeholder = "No role",
        hint = "People with this role use the Role access row."
    )]
    pub role: Option<String>,

    #[form(widget = Section, label = "Owner")]
    _section_owner: (),

    #[form(label = "Can view", widget = Checkbox, row = "owner_access")]
    pub owner_view: bool,

    #[form(label = "Can change", widget = Checkbox, row = "owner_access")]
    pub owner_change: bool,

    #[form(label = "Can open", widget = Checkbox, row = "owner_access")]
    pub owner_open: bool,

    #[form(widget = Section, label = "Role")]
    _section_role: (),

    #[form(label = "Can view", widget = Checkbox, row = "role_access")]
    pub role_view: bool,

    #[form(label = "Can change", widget = Checkbox, row = "role_access")]
    pub role_change: bool,

    #[form(label = "Can open", widget = Checkbox, row = "role_access")]
    pub role_open: bool,

    #[form(widget = Section, label = "Everyone else")]
    _section_other: (),

    #[form(
        label = "Can view",
        widget = Checkbox,
        row = "other_access",
        hint = "People who are not the owner and do not have the role above."
    )]
    pub other_view: bool,

    #[form(label = "Can change", widget = Checkbox, row = "other_access")]
    pub other_change: bool,

    #[form(label = "Can open", widget = Checkbox, row = "other_access")]
    pub other_open: bool,

    #[form(widget = Section, label = "Anyone")]
    _section_anyone: (),

    #[form(
        label = "Can view",
        widget = Checkbox,
        row = "anyone_access",
        hint = "Applies to every signed-in person, in addition to whichever row matches them."
    )]
    pub anyone_view: bool,

    #[form(label = "Can change", widget = Checkbox, row = "anyone_access")]
    pub anyone_change: bool,

    #[form(label = "Can open", widget = Checkbox, row = "anyone_access")]
    pub anyone_open: bool,

    #[form(
        label = "Also update everything inside this folder",
        widget = Checkbox,
        when = "is_directory",
        hint = "Replaces the owner, role, and access settings on this folder and every item inside it."
    )]
    pub apply_inside: bool,

    #[form(
        label = "Also update every item",
        widget = Checkbox,
        when = "is_root",
        hint = "Replaces the owner, role, and access settings on every file and folder."
    )]
    pub apply_all: bool,
}

#[html_form]
pub struct VNodeNameFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::{
        VNodeContentForm, VNodeContentFormField, VNodeForm, VNodeFormField, VNodeFormFlag,
        VNodeKind,
    };
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, HtmlKind};

    #[test]
    fn vnode_kind_variants() {
        let variants = VNodeKind::variants();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].value, "Directory");
        assert!(variants[0].fields.is_empty());
        assert_eq!(variants[1].value, "File");
        assert_eq!(variants[1].fields[0].name, "File");
    }

    #[test]
    fn vnode_form_create_renders_kind_radios() {
        let ctx = FormCtx::form::<VNodeForm>(CsrfToken::current())
            .flag(VNodeFormFlag::CreateMode, true)
            .kind::<VNodeKind>("File");
        let html = VNodeForm::render_inputs(&ctx).into_string();
        assert!(html.contains("type=\"radio\""), "{html}");
        assert!(html.contains("name=\"Kind\""), "{html}");
        assert!(html.contains("x-model=\"kind\""), "{html}");
        assert!(html.contains("name=\"ParentID\""), "{html}");
        assert!(html.contains("type=\"file\""), "{html}");
        assert!(html.contains("onchange="), "{html}");
        assert!(html.contains("[name=Name]"), "{html}");
        assert!(html.contains("files[0].name"), "{html}");
        // Hidden kind variants must disable required controls so HTML5 validation
        // does not fail with "invalid form control … is not focusable".
        assert!(
            html.contains("x-bind:disabled=\"!(kind === 'File')\""),
            "{html}"
        );
        assert!(
            html.contains("x-bind:disabled=\"!(kind === 'Directory')\""),
            "{html}"
        );
    }

    #[test]
    fn vnode_form_edit_locks_kind() {
        let ctx = FormCtx::form::<VNodeForm>(CsrfToken::current())
            .flag(VNodeFormFlag::CreateMode, false)
            .lock_kind(true)
            .kind::<VNodeKind>("Directory")
            .value(VNodeFormField::Name, "docs");
        let html = VNodeForm::render_inputs(&ctx).into_string();
        assert!(!html.contains("type=\"radio\""), "{html}");
        assert!(!html.contains("type=\"file\""), "{html}");
        assert!(!html.contains("name=\"ParentID\""), "{html}");
        assert!(html.contains("name=\"Name\""), "{html}");
    }

    #[test]
    fn vnode_content_form_renders_code_editor() {
        let ctx = FormCtx::form::<VNodeContentForm>(CsrfToken::current())
            .value(VNodeContentFormField::Content, "hello");
        let html = VNodeContentForm::render_inputs(&ctx).into_string();
        assert!(html.contains("data-code-editor-root"), "{html}");
        assert!(html.contains("name=\"Content\""), "{html}");
        assert!(html.contains("hello"), "{html}");
    }
}
