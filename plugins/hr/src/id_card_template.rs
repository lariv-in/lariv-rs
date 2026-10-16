//! Default Typst + Minijinja employee ID card template.

pub const DEFAULT_ID_CARD_TEMPLATE: &str = include_str!("templates/example_id_card.typ.tmpl");

pub const ID_CARD_FORMAT_HINT: &str = "\
Minijinja (Jinja2-style) template. Minijinja expands {% … %} and {{ … }; the result must be valid Typst source, which is then compiled to PDF. Leave blank to use the built-in KDS and Tagore portrait identity card.

Use Preview under the field to render the template with sample employee data before saving. Use Default Template to copy the shipped card into the field (you will be asked to confirm before the field is overwritten).

The context is one employee. Text fields are empty when unset. Pipe text through the typst filter before placing it in Typst markup ({{ Name|typst }}).

Identity:
• ID, EmployeeCode (assigned from the employee ID format), UserId, ManagerId, Manager (display name), Role (title of the employee's user role)
• Name, Mobile, Email
• Verified (true or false)
• CreatedAt / CreatedAtDisplay, UpdatedAt / UpdatedAtDisplay

Profile:
• FathersName, DateOfBirth, Gender and GenderCode, MaritalStatus and MaritalStatusCode, Nationality
• IsDisabled (true, false, or empty) and IsDisabledLabel (Yes, No, or empty), DisabilityType
• PhotographVnodeId (empty when there is no photo), BloodGroup, IdentificationMark
• PresentAddress, PresentPinCode, PermanentAddress, PermanentPinCode
• EmergencyContactName, EmergencyContactRelation, EmergencyContactMobile
• AadharVnodeId, PanVnodeId, PassportVnodeId (filesystem file ids; empty when unset)
• AccountHolderName, AccountNumber, AccountIfscCode, AccountType and AccountTypeCode
• Qualifications, DateOfJoining, ProbationEndDate, WorkStart, WorkEnd, BaseSalary, HourlyWage
• QrCode — filename of an SVG QR code in the Typst work directory. Empty only when the card is rendered without a signing key. The QR contains a JWT of EmployeeCode, Name, Role, Mobile, Email, Gender, and DateOfBirth. The JWT is signed with the private key and carries the public key in its header. Use #image(\"{{ QrCode }}\", …) when QrCode is set.

Dates use DD/MM/YYYY. Clock times use HH:MM:SS.

Template function:
• vnodeImage(vnode_id) — copy a filesystem file into the Typst work directory and return its filename for #image(\"…\"). Use it when PhotographVnodeId (or another file id) is set.

Write Typst markup (#set, #grid, #image, …) literally; only {% %} and {{ }} regions are evaluated by Minijinja.";
