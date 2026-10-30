//! The finger findings: fingers that curl at rest, and hands whose finger
//! bones sit bunched at the wrist. Neither stops a VRM file.

use super::{finding, list};
use crate::vrm::pose::RestPose;
use crate::vrm::{FindingLevel, VrmFinding};

/// A finger may curl this far in all, degrees, and still count as straight.
pub const FINGER_SLACK_DEG: f32 = 15.0;

/// "the right hand's", or "both hands'" for both.
fn hands(sides: &[String]) -> String {
    match sides {
        [one] => format!("The {one} hand's"),
        _ => "Both hands'".to_string(),
    }
}

pub(in crate::vrm) fn finger_findings(pose: &RestPose) -> Vec<VrmFinding> {
    let mut out = Vec::new();
    if let Some(most) = pose.finger_curl_deg.filter(|d| *d > FINGER_SLACK_DEG) {
        let mut curled: Vec<_> = pose
            .fingers
            .iter()
            .filter(|f| f.curl_deg > FINGER_SLACK_DEG)
            .collect();
        curled.sort_by(|a, b| b.curl_deg.total_cmp(&a.curl_deg));
        let worst: Vec<String> = curled
            .iter()
            .take(3)
            .map(|f| format!("{} ({:.0})", f.finger, f.curl_deg))
            .collect();
        out.push(finding(
            "fingers",
            FindingLevel::Warning,
            format!("Fingers curl up to {most:.0} degrees, VRM wants them straight"),
            Some(format!(
                "Most at {}. VRM's rest pose lays the four fingers straight along the hand and the thumb 45 degrees between the hand and the front, and animations retarget onto that. Normalising the rest pose straightens them as far as the hand's mesh holds; fused fingers keep some curl.",
                list(&worst)
            )),
        ));
    }
    if !pose.bunched_hands.is_empty() {
        out.push(finding(
            "finger-bones",
            FindingLevel::Warning,
            format!(
                "{} finger bones sit bunched at the wrist",
                hands(&pose.bunched_hands)
            ),
            Some("Every knuckle sits within 1.5% of the height of the wrist, as a broken auto rig leaves them, so those fingers cannot be measured or straightened and move with the hand as one piece. Rig the hands again, or place the finger joints by hand.".to_string()),
        ));
    }
    out
}
