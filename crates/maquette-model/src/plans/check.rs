//! The checks a canvas passes before it is written or after it is read:
//! its name, the limits, each node by its kind, and edges that join two of
//! its nodes. A refusal says what to fix.

use std::collections::BTreeSet;

use super::images::IMAGE_EXTENSIONS;
use super::{
    BY_MAX_CHARS, ID_MAX_CHARS, IMAGE_PATH_MAX_CHARS, LINK_ID_MAX_CHARS, MAX_COORD, MAX_EDGES,
    MAX_NODES, MAX_WIDTH, MIN_WIDTH, NAME_MAX_CHARS, NodeKind, PlanFile, PlanNode, TEXT_MAX_CHARS,
    TITLE_MAX_CHARS, URL_MAX_CHARS,
};
use crate::error::{CoreError, CoreResult};
use crate::project::is_windows_reserved;

fn invalid<T>(message: String) -> CoreResult<T> {
    Err(CoreError::Invalid(message))
}

fn is_item_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= ID_MAX_CHARS
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// True for the id of an asset or Lab direction a node may link: ASCII
/// letters, digits, dashes and underscores, as asset ids and direction
/// folders are made, so it can never name a path.
pub fn is_link_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= LINK_ID_MAX_CHARS
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Checks a canvas name: one line, not blank, not too long.
///
/// # Errors
///
/// [`CoreError::Invalid`] saying what to fix.
pub fn check_name(name: &str) -> CoreResult<()> {
    let name = super::trimmed(name);
    if name.is_empty() {
        return invalid("Give the canvas a name.".into());
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return invalid(format!(
            "Keep the canvas name under {NAME_MAX_CHARS} characters."
        ));
    }
    if name.chars().any(char::is_control) {
        return invalid("Keep the canvas name on one line.".into());
    }
    Ok(())
}

/// One folder or file name in a project relative path that works on every
/// system: not empty, not only dots, not ending in a dot or a space (Windows
/// drops those), and not a name Windows keeps for a device.
fn is_safe_part(part: &str) -> bool {
    !part.is_empty()
        && !part.chars().all(|c| c == '.')
        && !part.ends_with('.')
        && !part.ends_with(' ')
        && !part.chars().any(char::is_control)
        && !is_windows_reserved(part)
}

/// A project relative picture path: `/` separators, no `..`, not absolute,
/// no drive or share, and every part safe on Windows too.
pub fn is_project_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path.split('/').all(is_safe_part)
}

/// A picture path a canvas may keep: project relative, not too long, and a
/// kind of picture the canvas copies in.
fn is_picture_path(path: &str) -> bool {
    let ext = path
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    path.chars().count() <= IMAGE_PATH_MAX_CHARS
        && is_project_relative(path)
        && IMAGE_EXTENSIONS.contains(&ext.as_str())
}

fn is_web_address(url: &str) -> bool {
    let url = super::trimmed(url);
    url.len() <= URL_MAX_CHARS
        && (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() > "https://".len()
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Who made a node or edge: none, or a short one line name.
fn check_by(by: Option<&str>, what: &str) -> CoreResult<()> {
    match by {
        Some(by) if by.chars().count() > BY_MAX_CHARS || by.chars().any(char::is_control) => {
            invalid(format!(
                "{what} names who made it in over {BY_MAX_CHARS} characters or on several lines."
            ))
        }
        _ => Ok(()),
    }
}

/// The picture, address and link fields by kind: an image needs a picture,
/// a link an address, an asset node an asset id and a Lab direction node a
/// direction id, and each kind holds only its own. Asset and direction
/// nodes keep no title or text: their names are read from the project.
fn check_kind(node: &PlanNode) -> CoreResult<()> {
    let id = &node.id;
    let (image, url) = (node.image.as_deref(), node.url.as_deref());
    let (asset, direction) = (node.asset.as_deref(), node.direction.as_deref());
    match node.kind {
        NodeKind::Image if !image.is_some_and(is_picture_path) => {
            return invalid(format!(
                "The picture node {id} needs a picture in the project ({}).",
                IMAGE_EXTENSIONS.join(", ")
            ));
        }
        NodeKind::Link if !url.is_some_and(is_web_address) => {
            return invalid(format!(
                "The link node {id} needs an http or https address."
            ));
        }
        NodeKind::Asset if !asset.is_some_and(is_link_id) => {
            return invalid(format!(
                "The asset node {id} needs the id of an asset in the project."
            ));
        }
        NodeKind::Direction if !direction.is_some_and(is_link_id) => {
            return invalid(format!(
                "The Lab direction node {id} needs the id of a direction in the Lab."
            ));
        }
        NodeKind::Stop => match &node.stop {
            None => return invalid(format!("The stop node {id} needs its camera stop.")),
            Some(stop) => crate::layout::check_stop(stop)?,
        },
        _ => {}
    }
    let owned = [
        (
            image.is_some(),
            NodeKind::Image,
            "a picture node holds a picture",
        ),
        (
            url.is_some(),
            NodeKind::Link,
            "a link node holds an address",
        ),
        (
            asset.is_some(),
            NodeKind::Asset,
            "an asset node holds an asset",
        ),
        (
            direction.is_some(),
            NodeKind::Direction,
            "a Lab direction node holds a direction",
        ),
        (
            node.stop.is_some(),
            NodeKind::Stop,
            "a stop node holds a camera stop",
        ),
    ];
    if let Some((_, _, rule)) = owned
        .iter()
        .find(|(present, owner, _)| *present && node.kind != *owner)
    {
        return invalid(format!("Only {rule} ({id})."));
    }
    let linked = matches!(node.kind, NodeKind::Asset | NodeKind::Direction);
    if linked && !(node.title.is_empty() && node.text.is_empty()) {
        return invalid(format!(
            "The node {id} reads its name from the project, so it keeps no title or text."
        ));
    }
    if node.kind == NodeKind::Stop && !node.title.is_empty() {
        return invalid(format!(
            "The stop node {id} keeps its name in its camera stop, not in a title."
        ));
    }
    Ok(())
}

fn check_node(node: &PlanNode) -> CoreResult<()> {
    let id = &node.id;
    if !is_item_id(id) {
        return invalid(format!("The node id {id} is not a plain id."));
    }
    let within = |v: i32| v.unsigned_abs() <= MAX_COORD.unsigned_abs();
    if !within(node.x) || !within(node.y) {
        return invalid(format!("The node {id} sits too far out on the canvas."));
    }
    if let Some(w) = node.width
        && !(MIN_WIDTH..=MAX_WIDTH).contains(&w)
    {
        return invalid(format!(
            "The node {id} must be {MIN_WIDTH} to {MAX_WIDTH} pixels wide."
        ));
    }
    if node.title.chars().count() > TITLE_MAX_CHARS {
        return invalid(format!(
            "The node {id} has a title over {TITLE_MAX_CHARS} characters."
        ));
    }
    if node.text.chars().count() > TEXT_MAX_CHARS {
        return invalid(format!(
            "The node {id} holds over {TEXT_MAX_CHARS} characters."
        ));
    }
    check_by(node.by.as_deref(), &format!("The node {id}"))?;
    check_kind(node)
}

/// Checks a whole canvas: its name, the limits, every node, and edges that
/// join two nodes it has.
///
/// # Errors
///
/// [`CoreError::Invalid`] saying what to fix.
pub fn validate(plan: &PlanFile) -> CoreResult<()> {
    check_name(&plan.name)?;
    if plan.nodes.len() > MAX_NODES {
        return invalid(format!("A canvas holds at most {MAX_NODES} nodes."));
    }
    if plan.edges.len() > MAX_EDGES {
        return invalid(format!(
            "A canvas holds at most {MAX_EDGES} links between nodes."
        ));
    }
    let mut ids = BTreeSet::new();
    for node in &plan.nodes {
        check_node(node)?;
        if !ids.insert(node.id.as_str()) {
            return invalid(format!("Two nodes share the id {}.", node.id));
        }
    }
    super::camera::check_plan(plan)?;
    let mut edge_ids = BTreeSet::new();
    for edge in &plan.edges {
        let id = &edge.id;
        if !is_item_id(id) || !edge_ids.insert(id.as_str()) {
            return invalid(format!("The edge id {id} is not a plain, unique id."));
        }
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            return invalid(format!(
                "The edge {id} joins a node the canvas does not have."
            ));
        }
        if edge
            .label
            .as_ref()
            .is_some_and(|l| l.chars().count() > TITLE_MAX_CHARS)
        {
            return invalid(format!("The edge {id} has a label that is too long."));
        }
        check_by(edge.by.as_deref(), &format!("The edge {id}"))?;
    }
    Ok(())
}
