//! Request form structs for filesystem admin.

use crate::html_form::{
    Upload, html_form,
    widgets::{File, ForeignKey, Kind, Text},
};

// Keeps `Kind` in scope for `widget = Kind` (macro matches the path; not named in expansion).
const _: fn() = || {
    let _: Kind = Kind;
};

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
        #[form(label = "File", widget = File, required)]
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
        url = "/filesystem/select",
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

#[html_form(default)]
pub struct VNodeMultiUploadForm {
    #[form(
        label = "Destination Folder",
        widget = ForeignKey,
        url = "/filesystem/select",
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
        url = "/filesystem/select",
        swap_key = "fk-vnode-parent",
        display = "parent",
        placeholder = "Filesystem root"
    )]
    pub parent_id: Option<i64>,

    #[form(label = "Zip File", widget = File, accept = ".zip", required)]
    pub zip_file: Upload,
}

#[html_form]
pub struct VNodeNameFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::{VNodeForm, VNodeFormField, VNodeFormFlag, VNodeKind};
    use crate::html_form::{FormCtx, HtmlForm, HtmlKind};

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
        let ctx = FormCtx::form::<VNodeForm>()
            .flag(VNodeFormFlag::CreateMode, true)
            .kind::<VNodeKind>("File");
        let html = VNodeForm::render_inputs(&ctx).into_string();
        assert!(html.contains("type=\"radio\""), "{html}");
        assert!(html.contains("name=\"Kind\""), "{html}");
        assert!(html.contains("x-model=\"kind\""), "{html}");
        assert!(html.contains("name=\"ParentID\""), "{html}");
        assert!(html.contains("type=\"file\""), "{html}");
    }

    #[test]
    fn vnode_form_edit_locks_kind() {
        let ctx = FormCtx::form::<VNodeForm>()
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
}
