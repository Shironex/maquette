//! The rig doctor's tools: an agent can start and read it, and no tool
//! approves a joint move or lands a draft.

const NO_TOOL_MAY_DO: [&str; 4] = [
    "approve_rig_doctor",
    "land_rig_doctor",
    "decide_rig_doctor",
    "move_rig_joints",
];

#[test]
fn the_tools_can_start_and_read_the_doctor_but_never_approve() {
    let names = include_str!("../../tool-names.txt");
    for tool in [
        "start_rig_doctor",
        "run_rig_doctor",
        "get_rig_doctor",
        "list_rig_doctors",
    ] {
        assert!(names.lines().any(|l| l == tool), "{tool} is listed");
    }
    for tool in NO_TOOL_MAY_DO {
        assert!(!names.lines().any(|l| l == tool), "{tool} must not exist");
    }
}
