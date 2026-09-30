//! HR plugin user role names (seeded at startup).

pub const APPLICANT: &str = "applicant";
pub const PROBATION: &str = "probation";
pub const EMPLOYEE: &str = "employee";
pub const EX_EMPLOYEE: &str = "ex-employee";

pub const ALL: &[&str] = &[APPLICANT, PROBATION, EMPLOYEE, EX_EMPLOYEE];

pub struct SeededRole {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

pub const SEEDED: &[SeededRole] = &[
    SeededRole {
        name: APPLICANT,
        title: "Applicant",
        description: "Person who has applied and is being considered for a position.",
    },
    SeededRole {
        name: PROBATION,
        title: "Probation",
        description: "Employee serving a probationary period before confirmation.",
    },
    SeededRole {
        name: EMPLOYEE,
        title: "Employee",
        description: "Current employee of the organization.",
    },
    SeededRole {
        name: EX_EMPLOYEE,
        title: "Ex-Employee",
        description: "Former employee who has left the organization.",
    },
];
