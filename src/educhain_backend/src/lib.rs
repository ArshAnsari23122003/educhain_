use ic_cdk::{api, caller, query, update};
use candid::{CandidType, Deserialize};
use std::cell::RefCell;
use std::collections::HashMap;

type Principal = candid::Principal;

// Thread-local storages
thread_local! {
    static STUDENTS: RefCell<HashMap<Principal, StudentProfile>> = RefCell::new(HashMap::new());
    static COURSES: RefCell<HashMap<u64, Course>> = RefCell::new(HashMap::new());
    static ENROLLMENTS: RefCell<Vec<Enrollment>> = RefCell::new(Vec::new());
    static PENDING_REQUESTS: RefCell<Vec<CourseRequest>> = RefCell::new(Vec::new());
    static DAO_PROPOSALS: RefCell<Vec<DaoProposal>> = RefCell::new(Vec::new());
    static BANNED_INSTRUCTORS: RefCell<Vec<Principal>> = RefCell::new(Vec::new());
    static REMOVED_STUDENTS: RefCell<Vec<Principal>> = RefCell::new(Vec::new());
    static CONFIG: RefCell<TokenConfig> = RefCell::new(TokenConfig { reward_per_course: 10, cost_to_enroll: 5 });
}

// Data models
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct StudentProfile {
    name: String,
    roll_no: String,
    email: String,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Course {
    id: u64,
    title: String,
    description: String,
    instructor: Principal,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CourseRequest {
    id: u64,
    title: String,
    description: String,
    instructor: Principal,
    instructor_name: String,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Enrollment {
    course_id: u64,
    student: Principal,
    student_name: String,
    roll_no: String,
    passed: Option<bool>,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct DaoProposal {
    text: String,
    yes_votes: u32,
    no_votes: u32,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct TokenConfig {
    reward_per_course: u64,
    cost_to_enroll: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct PlatformStats {
    total_students: u64,
    total_courses: u64,
    certificates_issued: u64,
}

// Student functions
#[update]
fn update_student_profile(name: String, roll_no: String, email: String) -> String {
    let me = caller();
    STUDENTS.with(|s| s.borrow_mut().insert(me, StudentProfile { name, roll_no, email }));
    "Profile updated".into()
}

#[update]
fn enroll_in_course(course_id: u64) -> String {
    let me = caller();
    if REMOVED_STUDENTS.with(|r| r.borrow().contains(&me)) {
        return "Student removed.".into();
    }
    let profile = STUDENTS.with(|s| s.borrow().get(&me).cloned());
    if let Some(p) = profile {
        ENROLLMENTS.with(|e| {
            if e.borrow().iter().any(|en| en.course_id == course_id && en.student == me) {
                "Already enrolled.".into()
            } else {
                e.borrow_mut().push(Enrollment {
                    course_id,
                    student: me,
                    student_name: p.name,
                    roll_no: p.roll_no,
                    passed: None,
                });
                "Enrolled successfully.".into()
            }
        })
    } else {
        "Set profile first.".into()
    }
}

#[update]
fn drop_out_of_course(course_id: u64) -> String {
    let me = caller();
    ENROLLMENTS.with(|e| {
        let mut list = e.borrow_mut();
        let before = list.len();
        list.retain(|en| !(en.course_id == course_id && en.student == me));
        if list.len() < before { "Dropped out".into() } else { "Not enrolled".into() }
    })
}

#[query]
fn browse_courses() -> Vec<Course> {
    COURSES.with(|c| c.borrow().values().cloned().collect())
}

// Instructor functions
#[update]
fn request_new_course(title: String, description: String, instructor_name: String) -> CourseRequest {
    let req = CourseRequest {
        id: api::time() as u64,
        title,
        description,
        instructor: caller(),
        instructor_name,
    };
    PENDING_REQUESTS.with(|p| p.borrow_mut().push(req.clone()));
    req
}

#[query]
fn list_my_courses() -> Vec<Course> {
    let me = caller();
    COURSES.with(|c| c.borrow().values().filter(|x| x.instructor == me).cloned().collect())
}

#[query]
fn list_enrolled_students(course_id: u64) -> Vec<Enrollment> {
    ENROLLMENTS.with(|e| e.borrow().iter().filter(|en| en.course_id == course_id).cloned().collect())
}

#[update]
fn mark_pass(course_id: u64, student: Principal) -> String {
    mark_student(course_id, student, true)
}

#[update]
fn mark_fail(course_id: u64, student: Principal) -> String {
    mark_student(course_id, student, false)
}

fn mark_student(course_id: u64, student: Principal, pass: bool) -> String {
    ENROLLMENTS.with(|e| {
        if let Some(en) = e.borrow_mut().iter_mut().find(|en| en.course_id == course_id && en.student == student) {
            en.passed = Some(pass);
            if pass { "Marked pass".into() } else { "Marked fail".into() }
        } else {
            "Not found".into()
        }
    })
}

// Admin functions
#[update]
fn approve_course_request(id: u64) -> String {
    PENDING_REQUESTS.with(|p| {
        let mut list = p.borrow_mut();
        if let Some(pos) = list.iter().position(|x| x.id == id) {
            let req = list.remove(pos);
            COURSES.with(|c| c.borrow_mut().insert(req.id, Course {
                id: req.id,
                title: req.title,
                description: req.description,
                instructor: req.instructor,
            }));
            "Approved".into()
        } else { "Not found".into() }
    })
}

#[query]
fn list_pending_requests() -> Vec<CourseRequest> {
    PENDING_REQUESTS.with(|p| p.borrow().clone())
}

#[update]
fn add_dao_proposal(text: String) -> String {
    DAO_PROPOSALS.with(|d| d.borrow_mut().push(DaoProposal { text, yes_votes: 0, no_votes: 0 }));
    "Added".into()
}

#[update]
fn vote_on_proposal(index: u64, yes: bool) -> String {
    DAO_PROPOSALS.with(|d| {
        if let Some(p) = d.borrow_mut().get_mut(index as usize) {
            if yes { p.yes_votes += 1; } else { p.no_votes += 1; }
            "Voted".into()
        } else { "Not found".into() }
    })
}

#[query]
fn view_dao_proposals() -> Vec<DaoProposal> {
    DAO_PROPOSALS.with(|d| d.borrow().clone())
}

#[update]
fn ban_instructor(instructor: Principal) -> String {
    BANNED_INSTRUCTORS.with(|b| b.borrow_mut().push(instructor));
    "Banned".into()
}

#[update]
fn remove_student(student: Principal) -> String {
    REMOVED_STUDENTS.with(|r| r.borrow_mut().push(student));
    "Removed".into()
}

#[update]
fn set_token_reward(amount: u64) -> String {
    CONFIG.with(|c| c.borrow_mut().reward_per_course = amount);
    "Set".into()
}

#[update]
fn set_cost_to_enroll(amount: u64) -> String {
    CONFIG.with(|c| c.borrow_mut().cost_to_enroll = amount);
    "Set".into()
}

#[query]
fn get_platform_stats() -> PlatformStats {
    PlatformStats {
        total_students: STUDENTS.with(|s| s.borrow().len() as u64),
        total_courses: COURSES.with(|c| c.borrow().len() as u64),
        certificates_issued: ENROLLMENTS.with(|e| e.borrow().iter().filter(|en| en.passed == Some(true)).count() as u64),
    }
}

// Export Candid
ic_cdk::export_candid!();
