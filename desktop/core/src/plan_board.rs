// The plan board's rules (experiment planBoard, runner/src/experiments/planBoard.md):
// a thread's plan as boxes, groups and labelled arrows, drawn beside the thread
// by desktop/gpui/src/plan_board.rs. The runner parses the agent's Mermaid
// (runner/src/boards.ts) and sends the graph in `board.updated` and
// `board.get`, so nothing here reads Mermaid.
//
// What's here: reading the board, a simple layered layout (ranks follow the
// plan's direction; boxes Wayne dragged keep their place, keyed by id, across
// versions; Tidy lays it out again), hit tests, a selection as composer chips,
// the plan's state (draft, approved, changed since approved), the brief a box
// sends, each box's status from its thread's summary, and when to offer Ask
// planner to review. Pele draws Mermaid in replies, but doesn't hand out node
// positions, and board boxes carry badges and menus Pele's don't, so the
// layout is our own (PRD decision 3).
use crate::inbox::ThreadSummary;
pub use crate::review_canvas::{smooth_wheel_zoom, wheel_zoom, Camera, Rect};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const EXPERIMENT: &str = "planBoard";
pub const MOCKUPS: &str = "planBoardMockups";
pub const COORDINATORS: &str = "planBoardCoordinators";

// ---------------------------------------------------------------- the board

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    /// box, db or round.
    #[serde(default)]
    pub shape: String,
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
    /// `---`: a line with no arrowhead.
    #[serde(default)]
    pub line: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub skipped: u32,
}

impl Graph {
    pub fn top_to_bottom(&self) -> bool {
        self.direction == "TB"
    }
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub near: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Approved {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub at: String,
}

/// Where a box went: a nested thread, or (planBoardCoordinators) a to-do on
/// the project's coordinator.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    #[serde(default)]
    pub kind: Option<String>,
    /// The nested thread, or the coordinator's conversation.
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub harness: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub plan_version: u32,
    #[serde(default)]
    pub history: Vec<Value>,
}

impl Link {
    pub fn is_coordinator(&self) -> bool {
        self.kind.as_deref() == Some("coordinator")
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub saved_at: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    #[serde(default)]
    pub thread_id: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub saved_at: String,
    #[serde(default)]
    pub mermaid: String,
    #[serde(default)]
    pub graph: Graph,
    #[serde(default)]
    pub note: Option<Note>,
    #[serde(default)]
    pub changed: Vec<String>,
    #[serde(default)]
    pub approved: Option<Approved>,
    #[serde(default)]
    pub positions: BTreeMap<String, (f32, f32)>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub links: BTreeMap<String, Link>,
    #[serde(default)]
    pub versions: Vec<VersionInfo>,
}

impl Board {
    /// Leniently, as every client reads the runner: a board with no
    /// version or no boxes isn't one.
    pub fn decode(v: &Value) -> Option<Board> {
        let b: Board = serde_json::from_value(v.clone()).ok()?;
        (b.version > 0 && !b.thread_id.is_empty()).then_some(b)
    }

    pub fn state(&self) -> PlanState {
        match &self.approved {
            Some(a) if a.version == self.version => PlanState::Approved(self.version),
            Some(a) => PlanState::Changed {
                approved: a.version,
                latest: self.version,
            },
            None => PlanState::Draft(self.version),
        }
    }
}

// ---------------------------------------------------------------- plan state

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanState {
    Draft(u32),
    Approved(u32),
    /// The agent wrote a new version after Wayne approved one: a draft again,
    /// with every box's Send to… greyed until he approves the new one.
    Changed {
        approved: u32,
        latest: u32,
    },
}

impl PlanState {
    /// The header's words: "Draft v1", "Approved v2", "Changed since you approved (v2 → v3)".
    pub fn words(&self) -> String {
        match self {
            PlanState::Draft(v) => format!("Draft v{v}"),
            PlanState::Approved(v) => format!("Approved v{v}"),
            PlanState::Changed { approved, latest } => {
                format!("Changed since you approved (v{approved} → v{latest})")
            }
        }
    }
    pub fn approved(&self) -> bool {
        matches!(self, PlanState::Approved(_))
    }
}

/// The board's one line on what to do next, under its bar, so someone new
/// to it isn't left guessing (Wayne, 26 Nov: "make sense for newbies").
pub fn next_step(board: &Board) -> &'static str {
    match board.state() {
        PlanState::Draft(_) => "Draft: click a box to chat or open Screens now. Press Approve plan when ready to send boxes to build.",
        PlanState::Changed { .. } => "Plan updated after approval: review Changed boxes, then press Approve plan again to unlock Send.",
        PlanState::Approved(_) if board.links.is_empty() => {
            "Approved: press Send on any box to start its build thread, or tell this thread \"build it\"."
        }
        PlanState::Approved(_) => "In progress: click a box's status badge to open its thread. Press Ask planner to review once all finish.",
    }
}

/// Whether a box's Send to… (or Send again) can be used, and the hint when not.
/// Approval is per version: every box greys together.
pub fn send_allowed(
    board: &Board,
    node: &str,
    status: Option<BoxStatus>,
) -> Result<(), &'static str> {
    if board.graph.node(node).is_none() {
        return Err("This box isn't in the plan any more");
    }
    if !board.state().approved() {
        return Err(match board.state() {
            PlanState::Changed { .. } => "The plan changed: approve it again first",
            _ => "Approve the plan first",
        });
    }
    match status {
        // Already sent and still going, or done: one box, one thread.
        Some(BoxStatus::Failed | BoxStatus::Archived) | None => Ok(()),
        Some(_) => Err("Already sent"),
    }
}

// ---------------------------------------------------------------- layout

pub const BOX_H: f32 = 56.;
pub const RANK_GAP: f32 = 110.;
pub const ROW_GAP: f32 = 52.;
pub const GROUP_PAD: f32 = 18.;
pub const GROUP_HEAD: f32 = 26.;

/// A box's width from its label, so long labels wrap less.
pub fn box_width(label: &str) -> f32 {
    (label.chars().count() as f32 * 7.4 + 40.).clamp(120., 230.)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub start: (f32, f32),
    pub c1: (f32, f32),
    pub c2: (f32, f32),
    pub end: (f32, f32),
}

impl Curve {
    pub fn at(&self, t: f32) -> (f32, f32) {
        let u = 1. - t;
        let (a, b, c, d) = (self.start, self.c1, self.c2, self.end);
        (
            u * u * u * a.0 + 3. * u * u * t * b.0 + 3. * u * t * t * c.0 + t * t * t * d.0,
            u * u * u * a.1 + 3. * u * u * t * b.1 + 3. * u * t * t * c.1 + t * t * t * d.1,
        )
    }
    pub fn mid(&self) -> (f32, f32) {
        self.at(0.5)
    }
    /// The nearest a point comes to the curve, sampled.
    pub fn distance(&self, p: (f32, f32)) -> f32 {
        (0..=24)
            .map(|i| {
                let q = self.at(i as f32 / 24.);
                ((q.0 - p.0).powi(2) + (q.1 - p.1).powi(2)).sqrt()
            })
            .fold(f32::MAX, f32::min)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    /// Every box, by id, in board units.
    pub boxes: Vec<(String, Rect)>,
    /// Every group, outermost first, round its members.
    pub groups: Vec<(String, Rect)>,
    /// One per edge, in the plan's order.
    pub edges: Vec<Curve>,
}

impl Layout {
    pub fn rect(&self, id: &str) -> Option<Rect> {
        self.boxes.iter().find(|(i, _)| i == id).map(|(_, r)| *r)
    }
    pub fn group_rect(&self, id: &str) -> Option<Rect> {
        self.groups.iter().find(|(i, _)| i == id).map(|(_, r)| *r)
    }
    /// The whole board, for fit.
    pub fn bounds(&self) -> Rect {
        let mut all = self
            .boxes
            .iter()
            .map(|(_, r)| *r)
            .chain(self.groups.iter().map(|(_, r)| *r));
        let first = all.next().unwrap_or(Rect::new(0., 0., 1., 1.));
        all.fold(first, |a, r| a.union(&r))
    }
}

/// Ranks by longest path from a box with nothing coming in, following the
/// arrows; an arrow that would close a loop is left out of the ranking.
fn ranks(g: &Graph) -> HashMap<String, usize> {
    let ids: Vec<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut out: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &g.edges {
        if e.from != e.to && ids.contains(&e.from.as_str()) && ids.contains(&e.to.as_str()) {
            out.entry(e.from.as_str()).or_default().push(e.to.as_str());
        }
    }
    // Back edges by a depth-first walk in plan order, so a loop ranks the
    // same way every time.
    let mut state: HashMap<&str, u8> = HashMap::new();
    let mut keep: Vec<(&str, &str)> = Vec::new();
    fn walk<'a>(
        n: &'a str,
        out: &HashMap<&'a str, Vec<&'a str>>,
        state: &mut HashMap<&'a str, u8>,
        keep: &mut Vec<(&'a str, &'a str)>,
    ) {
        state.insert(n, 1);
        for &m in out.get(n).map(Vec::as_slice).unwrap_or(&[]) {
            match state.get(m).copied().unwrap_or(0) {
                0 => {
                    keep.push((n, m));
                    walk(m, out, state, keep);
                }
                2 => keep.push((n, m)),
                _ => {} // back edge
            }
        }
        state.insert(n, 2);
    }
    for &n in &ids {
        if state.get(n).copied().unwrap_or(0) == 0 {
            walk(n, &out, &mut state, &mut keep);
        }
    }
    let mut rank: HashMap<String, usize> = ids.iter().map(|i| (i.to_string(), 0)).collect();
    // Longest path on a DAG: relax until nothing changes (at most n rounds).
    for _ in 0..ids.len() {
        let mut moved = false;
        for &(a, b) in &keep {
            let want = rank[a] + 1;
            if rank[b] < want {
                rank.insert(b.to_string(), want);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    rank
}

/// The group a box sits in, then that group's parents, outermost last.
fn group_chain<'a>(g: &'a Graph, node: &'a Node) -> Vec<&'a str> {
    let mut chain = Vec::new();
    let mut at = node.group.as_deref();
    while let Some(id) = at {
        if chain.contains(&id) {
            break;
        }
        chain.push(id);
        at = g.group(id).and_then(|x| x.parent.as_deref());
    }
    chain
}

/// Lays the plan out in ranks. `positions` are the boxes Wayne dragged (top
/// left corners), kept by id; every other box goes where the layout puts it.
pub fn layout(g: &Graph, positions: &BTreeMap<String, (f32, f32)>) -> Layout {
    let rank = ranks(g);
    let tb = g.top_to_bottom();
    let depth = rank.values().copied().max().unwrap_or(0);
    let group_order: HashMap<&str, usize> = g
        .groups
        .iter()
        .enumerate()
        .map(|(i, x)| (x.id.as_str(), i))
        .collect();
    let plan_order: HashMap<&str, usize> = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    // Rank by rank, boxes ordered by where their predecessors sit (one
    // barycentre pass), then by group so a group's boxes stay together, then
    // plan order: the same plan always comes out the same.
    let mut slot: HashMap<String, f32> = HashMap::new();
    let mut lanes: Vec<Vec<&Node>> = vec![Vec::new(); depth + 1];
    for n in &g.nodes {
        lanes[rank[&n.id]].push(n);
    }
    for lane in lanes.iter_mut() {
        let key = |n: &Node| {
            let preds: Vec<f32> = g
                .edges
                .iter()
                .filter(|e| e.to == n.id)
                .filter_map(|e| slot.get(&e.from).copied())
                .collect();
            let bary = if preds.is_empty() {
                plan_order[n.id.as_str()] as f32
            } else {
                preds.iter().sum::<f32>() / preds.len() as f32
            };
            let grp = n
                .group
                .as_deref()
                .and_then(|x| group_order.get(x).copied())
                .unwrap_or(usize::MAX);
            (grp, bary, plan_order[n.id.as_str()])
        };
        lane.sort_by(|a, b| {
            let (ga, ba, pa) = key(a);
            let (gb, bb, pb) = key(b);
            ga.cmp(&gb).then(ba.total_cmp(&bb)).then(pa.cmp(&pb))
        });
        for (i, n) in lane.iter().enumerate() {
            slot.insert(n.id.clone(), i as f32);
        }
    }
    // Rank widths in the flow direction: the widest box in each rank.
    let along: Vec<f32> = lanes
        .iter()
        .map(|lane| {
            lane.iter()
                .map(|n| if tb { BOX_H } else { box_width(&n.label) })
                .fold(0., f32::max)
        })
        .collect();
    let extra_for_groups = GROUP_PAD * 2.;
    let mut boxes = Vec::new();
    let mut flow = 0.;
    for (r, lane) in lanes.iter().enumerate() {
        let mut across = 0.;
        let mut prev_group: Option<&str> = None;
        for n in lane {
            let w = box_width(&n.label);
            let (bw, bh) = (w, BOX_H);
            // Room between boxes of different groups for the groups' frames.
            let grp = n.group.as_deref();
            if prev_group.is_some() && grp != prev_group {
                across += extra_for_groups + GROUP_HEAD;
            }
            prev_group = grp;
            let (x, y) = if tb {
                (across, flow + (along[r] - bh) / 2.)
            } else {
                (flow + (along[r] - bw) / 2., across + GROUP_HEAD)
            };
            let at = positions.get(&n.id).copied().unwrap_or((x, y));
            boxes.push((n.id.clone(), Rect::new(at.0, at.1, bw, bh)));
            across += if tb { bw } else { bh } + ROW_GAP;
        }
        flow += along[r] + RANK_GAP;
    }
    // Groups round their boxes and inner groups, innermost first so a parent
    // takes in its children's frames.
    let mut groups: Vec<(String, Rect)> = Vec::new();
    let mut depth_of: Vec<(usize, &Group)> = g
        .groups
        .iter()
        .map(|x| {
            let mut d = 0;
            let mut at = x.parent.as_deref();
            let mut seen = HashSet::new();
            while let Some(p) = at {
                if !seen.insert(p) {
                    break;
                }
                d += 1;
                at = g.group(p).and_then(|y| y.parent.as_deref());
            }
            (d, x)
        })
        .collect();
    depth_of.sort_by_key(|d| std::cmp::Reverse(d.0));
    for (_, grp) in depth_of {
        let members = g
            .nodes
            .iter()
            .filter(|n| group_chain(g, n).contains(&grp.id.as_str()))
            .filter_map(|n| boxes.iter().find(|(i, _)| *i == n.id).map(|(_, r)| *r));
        let inner = groups
            .iter()
            .filter(|(id, _)| {
                g.group(id).and_then(|x| x.parent.as_deref()) == Some(grp.id.as_str())
            })
            .map(|(_, r)| *r);
        let mut all = members.chain(inner);
        let Some(first) = all.next() else { continue };
        let r = all.fold(first, |a, b| a.union(&b));
        groups.push((
            grp.id.clone(),
            Rect::new(
                r.x - GROUP_PAD,
                r.y - GROUP_PAD - GROUP_HEAD,
                r.w + GROUP_PAD * 2.,
                r.h + GROUP_PAD * 2. + GROUP_HEAD,
            ),
        ));
    }
    groups.reverse();
    let edges = g
        .edges
        .iter()
        .map(|e| {
            let a = boxes.iter().find(|(i, _)| *i == e.from).map(|(_, r)| *r);
            let b = boxes.iter().find(|(i, _)| *i == e.to).map(|(_, r)| *r);
            match (a, b) {
                (Some(a), Some(b)) => curve_between(&a, &b, tb),
                _ => Curve {
                    start: (0., 0.),
                    c1: (0., 0.),
                    c2: (0., 0.),
                    end: (0., 0.),
                },
            }
        })
        .collect();
    Layout {
        boxes,
        groups,
        edges,
    }
}

/// An arrow from one box to another: out of the side facing it, curving in.
pub fn curve_between(a: &Rect, b: &Rect, tb: bool) -> Curve {
    let (ac, bc) = (a.center(), b.center());
    let (dx, dy) = (bc.0 - ac.0, bc.1 - ac.1);
    // Mostly along the flow: leave by the facing side; otherwise by the
    // side the other box is on.
    let horizontal = if tb {
        dx.abs() > dy.abs() * 1.6
    } else {
        dx.abs() * 1.6 >= dy.abs()
    };
    if horizontal {
        let (sx, ex) = if dx >= 0. {
            (a.right(), b.x)
        } else {
            (a.x, b.right())
        };
        let k = ((ex - sx).abs() * 0.45).max(30.);
        let s = if dx >= 0. { 1. } else { -1. };
        Curve {
            start: (sx, ac.1),
            c1: (sx + k * s, ac.1),
            c2: (ex - k * s, bc.1),
            end: (ex, bc.1),
        }
    } else {
        let (sy, ey) = if dy >= 0. {
            (a.bottom(), b.y)
        } else {
            (a.y, b.bottom())
        };
        let k = ((ey - sy).abs() * 0.45).max(30.);
        let s = if dy >= 0. { 1. } else { -1. };
        Curve {
            start: (ac.0, sy),
            c1: (ac.0, sy + k * s),
            c2: (bc.0, ey - k * s),
            end: (bc.0, ey),
        }
    }
}

// ---------------------------------------------------------------- hits

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Hit {
    Box(String),
    Group(String),
    /// An arrow, by its place in the plan's edges.
    Arrow(usize),
}

/// What's under a point on the board (board units). Boxes first, then
/// arrows, then the innermost group. `scale` is the camera's, so an arrow is
/// as easy to hit at any zoom.
pub fn hit(layout: &Layout, at: (f32, f32), scale: f32) -> Option<Hit> {
    if let Some((id, _)) = layout.boxes.iter().rev().find(|(_, r)| r.contains(at)) {
        return Some(Hit::Box(id.clone()));
    }
    let reach = 7. / scale.max(0.05);
    if let Some((i, _)) = layout
        .edges
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.distance(at)))
        .filter(|(_, d)| *d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
    {
        return Some(Hit::Arrow(i));
    }
    layout
        .groups
        .iter()
        .rev()
        .find(|(_, r)| r.contains(at))
        .map(|(id, _)| Hit::Group(id.clone()))
}

// ---------------------------------------------------------------- chips

/// What a hit is called, for its chip and its accessibility label.
pub fn hit_label(board: &Board, h: &Hit) -> String {
    let g = &board.graph;
    match h {
        Hit::Box(id) => g
            .node(id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| id.clone()),
        Hit::Group(id) => g
            .group(id)
            .map(|x| x.label.clone())
            .unwrap_or_else(|| id.clone()),
        Hit::Arrow(i) => g
            .edges
            .get(*i)
            .map(|e| {
                let name = |id: &str| {
                    g.node(id)
                        .map(|n| n.label.clone())
                        .unwrap_or_else(|| id.to_owned())
                };
                match &e.label {
                    Some(l) => format!("{} → {} ({l})", name(&e.from), name(&e.to)),
                    None => format!("{} → {}", name(&e.from), name(&e.to)),
                }
            })
            .unwrap_or_default(),
    }
}

/// The id after `#` in a chip's link: a box's or group's own id, an arrow's
/// as `from->to`.
pub fn hit_ref(board: &Board, h: &Hit) -> String {
    match h {
        Hit::Box(id) | Hit::Group(id) => id.clone(),
        Hit::Arrow(i) => board
            .graph
            .edges
            .get(*i)
            .map(|e| format!("{}->{}", e.from, e.to))
            .unwrap_or_default(),
    }
}

/// A selection as the composer's chip: a Markdown link the agent reads the
/// box from (`board:<threadId>#<nodeId>`), drawn as `◻ KPI cards`.
pub fn chip_markdown(board: &Board, h: &Hit) -> String {
    let label = hit_label(board, h).replace('[', "\\[").replace(']', "\\]");
    format!("[{label}](board:{}#{})", board.thread_id, hit_ref(board, h))
}

/// The thread and the box a `board:` link names.
pub fn parse_board_link(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("board:")?;
    let (thread, node) = rest.split_once('#')?;
    (!thread.is_empty() && !node.is_empty()).then_some((thread, node))
}

// ---------------------------------------------------------------- the brief

/// The fixed header the runner also writes (runner/src/boards.ts
/// `briefHeader`): the ids let the box's thread call read_board.
pub fn brief_header(board: &Board, node: &Node) -> String {
    let short: String = board.thread_id.chars().take(8).collect();
    let v = board
        .approved
        .as_ref()
        .map(|a| a.version)
        .unwrap_or(board.version);
    format!(
        "Plan board: thread {short}, box {} (\"{}\"), plan v{v} (approved).",
        node.id, node.label
    )
}

/// "Use these skills: /a, /b" for Claude, "$a, $b" for Codex (its own way of
/// naming a skill mid-message); nothing with no skills.
pub fn skills_line(skills: &[String], provider: &str) -> Option<String> {
    if skills.is_empty() {
        return None;
    }
    let mark = if provider == "codex" { "$" } else { "/" };
    Some(format!(
        "Use these skills: {}",
        skills
            .iter()
            .map(|s| format!("{mark}{s}"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// The brief Send shows, editable, before Start: the same text the
/// runner would write for this box (runner/src/boards.ts `buildBrief`).
pub fn brief(board: &Board, node_id: &str, provider: &str, skills: &[String]) -> Option<String> {
    brief_with_screens(board, node_id, provider, skills, &[])
}

/// Same as `brief`, plus any screens picked for this box so the build thread
/// knows which mockup take Wayne chose.
pub fn brief_with_screens(
    board: &Board,
    node_id: &str,
    provider: &str,
    skills: &[String],
    picked_screens: &[Frame],
) -> Option<String> {
    let g = &board.graph;
    let node = g.node(node_id)?;
    let name = |id: &str| {
        g.node(id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| id.to_owned())
    };
    let mut neighbours: Vec<String> = g
        .edges
        .iter()
        .filter(|e| e.to == node_id)
        .map(|e| match &e.label {
            Some(l) => format!("{} ({l}) → this", name(&e.from)),
            None => format!("{} → this", name(&e.from)),
        })
        .collect();
    neighbours.extend(
        g.edges
            .iter()
            .filter(|e| e.from == node_id)
            .map(|e| match &e.label {
                Some(l) => format!("this → {} ({l})", name(&e.to)),
                None => format!("this → {}", name(&e.to)),
            }),
    );
    let group = node
        .group
        .as_deref()
        .and_then(|x| g.group(x))
        .map(|x| x.label.clone());
    let mut lines = vec![brief_header(board, node)];
    if let Some(s) = skills_line(skills, provider) {
        lines.push(s);
    }
    lines.push(String::new());
    lines.push(match group {
        Some(gr) => format!(
            "You're building \"{}\" from that plan, in the group \"{gr}\".",
            node.label
        ),
        None => format!("You're building \"{}\" from that plan.", node.label),
    });
    if !neighbours.is_empty() {
        lines.push(format!("Neighbours: {}.", neighbours.join("; ")));
    }
    for f in picked_screens
        .iter()
        .filter(|f| f.node == node_id && f.picked)
    {
        let source = f
            .route
            .as_deref()
            .map(|r| format!("route {r}"))
            .or_else(|| f.path.as_deref().map(|p| format!("file {p}")))
            .unwrap_or_default();
        if source.is_empty() {
            lines.push(format!(
                "Picked screen: \"{}\" (variant {}, {}, v{}).",
                f.title, f.variant, f.viewport, f.version
            ));
        } else {
            lines.push(format!(
                "Picked screen: \"{}\" (variant {}, {}, v{}): {source}",
                f.title, f.variant, f.viewport, f.version
            ));
        }
    }
    let note = board
        .note
        .as_ref()
        .map(|n| n.text.clone())
        .or_else(|| g.notes.first().cloned());
    if let Some(n) = note.filter(|n| !n.is_empty()) {
        lines.push(format!("Planner's note: {n}"));
    }
    lines.push(String::new());
    lines.push(
        "When you finish, say what you did. If you need Wayne, ask him; your box shows it."
            .to_owned(),
    );
    Some(lines.join("\n"))
}

/// Ready-to-send message for the Screens drawer's one-click Generate 3 takes button.
pub fn ask_screens_message(board: &Board, node_id: &str) -> String {
    format!(
        "{} Make three screen takes (A, B, and C) for this box so I can compare them on the plan board.",
        chip_markdown(board, &Hit::Box(node_id.to_owned()))
    )
}

/// Board skills a login can't run, left out of its brief (the sheet says
/// "not on this login"). `available` is that login's skill names.
pub fn skills_for_login(board: &[String], available: &[String]) -> (Vec<String>, Vec<String>) {
    board
        .iter()
        .cloned()
        .partition(|s| available.iter().any(|a| a == s))
}

// ---------------------------------------------------------------- status

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoxStatus {
    /// With its latest activity line, if any.
    Working(String),
    NeedsYou,
    Done,
    Failed,
    Archived,
    /// The link is there but its thread isn't in the list (another runner offline).
    CantSee,
}

impl BoxStatus {
    pub fn word(&self) -> &'static str {
        match self {
            BoxStatus::Working(_) => "Working",
            BoxStatus::NeedsYou => "Needs you",
            BoxStatus::Done => "Done",
            BoxStatus::Failed => "Failed",
            BoxStatus::Archived => "Archived",
            BoxStatus::CantSee => "Can't see it",
        }
    }
    /// Done or Failed: nothing more will happen unless someone acts.
    pub fn settled(&self) -> bool {
        matches!(
            self,
            BoxStatus::Done | BoxStatus::Failed | BoxStatus::Archived
        )
    }
}

/// A box's status, from its thread's summary alone, through the inbox's own
/// rules, so the badge, the sidebar and notifications never disagree.
pub fn status(t: Option<&ThreadSummary>) -> BoxStatus {
    let Some(t) = t else {
        return BoxStatus::CantSee;
    };
    if t.is_archived() {
        return BoxStatus::Archived;
    }
    if t.on_card() || t.waiting_on_mike() || t.needs_answer() {
        return BoxStatus::NeedsYou;
    }
    if t.busy() || t.working() {
        let line = t
            .preview
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("");
        let short: String = line.chars().take(48).collect();
        return BoxStatus::Working(if line.chars().count() > 48 {
            format!("{short}…")
        } else {
            short
        });
    }
    if t.status == "error" {
        return BoxStatus::Failed;
    }
    BoxStatus::Done
}

/// Phase 3 (planBoardCoordinators): a coordinator to-do as the runner lists
/// it (Mike's coordinators: todo, working, blocked, review, done).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorTask {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub state: String,
    /// The coordinator asked Wayne about this to-do.
    #[serde(default)]
    pub asked_wayne: bool,
    #[serde(default)]
    pub failed: bool,
    #[serde(default)]
    pub archived: bool,
}

/// The same badges for a box handed to the coordinator (PRD §16.2's table).
/// `coordinator` is the coordinator conversation's summary; with no task or
/// no coordinator to read, Can't see it.
pub fn coordinator_status(
    task: Option<&CoordinatorTask>,
    coordinator: Option<&ThreadSummary>,
) -> BoxStatus {
    let (Some(task), Some(c)) = (task, coordinator) else {
        return BoxStatus::CantSee;
    };
    if task.archived {
        return BoxStatus::Archived;
    }
    if task.failed {
        return BoxStatus::Failed;
    }
    match task.state.as_str() {
        "done" => BoxStatus::Done,
        // Needs you only when the coordinator has asked Wayne, which the
        // inbox shows on the coordinator's own row.
        "blocked" if task.asked_wayne || c.waiting_on_mike() || c.on_card() => BoxStatus::NeedsYou,
        "todo" => BoxStatus::Working("Queued for the coordinator".to_owned()),
        "review" => BoxStatus::Working("The coordinator is checking it".to_owned()),
        _ => BoxStatus::Working(String::new()),
    }
}

/// Every box's status, by id, for the boxes that were sent.
pub fn statuses(
    board: &Board,
    summary: impl Fn(&str) -> Option<ThreadSummary>,
    tasks: impl Fn(&str) -> Option<CoordinatorTask>,
) -> BTreeMap<String, BoxStatus> {
    board
        .links
        .iter()
        .map(|(id, link)| {
            let s = if link.is_coordinator() {
                let task = link.task_id.as_deref().and_then(&tasks);
                let coord = link.thread_id.as_deref().and_then(&summary);
                coordinator_status(task.as_ref(), coord.as_ref())
            } else {
                status(link.thread_id.as_deref().and_then(&summary).as_ref())
            };
            (id.clone(), s)
        })
        .collect()
}

/// One row in the board's live Activity tray: either the planner thread itself
/// (when working, waiting on you, or just asked from the board) or a sent box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityRow {
    /// Box id for a sent box, or None for the planner thread itself.
    pub node_id: Option<String>,
    /// Box label, or "Planner" for the board's own thread.
    pub label: String,
    /// Where it runs: "Claude", "Codex", or "Coordinator".
    pub where_name: String,
    /// Thread id to open on click, when known.
    pub thread_id: Option<String>,
    /// Linked thread's title, shown when it adds context beyond `label`.
    pub thread_title: String,
    /// Live status through the same inbox rules as box badges.
    pub status: BoxStatus,
    /// Short plain words on what it is doing right now.
    pub detail: String,
}

/// Counts and toolbar pill wording for the board's live Activity tray.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivitySummary {
    pub running: usize,
    pub needs_you: usize,
    pub done: usize,
    pub failed: usize,
    pub label: String,
}

fn status_detail(st: &BoxStatus) -> String {
    match st {
        BoxStatus::Working(line) if !line.is_empty() => line.clone(),
        BoxStatus::Working(_) => "Working on it".to_owned(),
        BoxStatus::NeedsYou => "Waiting for your answer".to_owned(),
        BoxStatus::Done => "Finished".to_owned(),
        BoxStatus::Failed => "Stopped with an error".to_owned(),
        BoxStatus::Archived => "Thread archived".to_owned(),
        BoxStatus::CantSee => "Thread is offline or on another Mac".to_owned(),
    }
}

fn status_rank(st: &BoxStatus) -> u8 {
    match st {
        BoxStatus::NeedsYou => 0,
        BoxStatus::Working(_) => 1,
        BoxStatus::Failed => 2,
        BoxStatus::Done => 3,
        BoxStatus::Archived | BoxStatus::CantSee => 4,
    }
}

/// Live rows for the board's Activity tray: the planner thread when active
/// or just asked from the board, plus every sent box sorted by urgency.
pub fn activity_rows(
    board: &Board,
    planner: Option<&ThreadSummary>,
    planner_note: Option<&str>,
    statuses: &BTreeMap<String, BoxStatus>,
    summary: impl Fn(&str) -> Option<ThreadSummary>,
) -> Vec<ActivityRow> {
    let mut rows: Vec<(u8, usize, ActivityRow)> = Vec::new();
    if let Some(p) = planner {
        let p_status = status(Some(p));
        let working = matches!(p_status, BoxStatus::Working(_));
        if working || planner_note.is_some() {
            let shown_status = match (&p_status, planner_note) {
                (BoxStatus::Working(line), Some(note)) if line.is_empty() => {
                    BoxStatus::Working(note.to_owned())
                }
                (BoxStatus::Working(_), _) => p_status.clone(),
                (_, Some(note)) => BoxStatus::Working(note.to_owned()),
                _ => p_status.clone(),
            };
            let where_name = if p.provider == "codex" {
                "Codex"
            } else {
                "Claude"
            };
            rows.push((
                status_rank(&shown_status),
                0,
                ActivityRow {
                    node_id: None,
                    label: "Planner".to_owned(),
                    where_name: where_name.to_owned(),
                    thread_id: Some(p.id.clone()),
                    thread_title: p.title.clone(),
                    detail: status_detail(&shown_status),
                    status: shown_status,
                },
            ));
        }
    }
    let order: HashMap<&str, usize> = board
        .graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i + 1))
        .collect();
    for (id, link) in &board.links {
        let st = statuses.get(id).cloned().unwrap_or(BoxStatus::CantSee);
        let label = board
            .graph
            .node(id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| id.clone());
        let where_name = if link.is_coordinator() {
            "Coordinator"
        } else if link.harness.as_deref() == Some("codex") {
            "Codex"
        } else {
            "Claude"
        };
        let thread_title = link
            .thread_id
            .as_deref()
            .and_then(&summary)
            .map(|t| t.title)
            .unwrap_or_default();
        let rank = status_rank(&st);
        let pos = order.get(id.as_str()).copied().unwrap_or(usize::MAX);
        rows.push((
            rank,
            pos,
            ActivityRow {
                node_id: Some(id.clone()),
                label,
                where_name: where_name.to_owned(),
                thread_id: link.thread_id.clone(),
                thread_title,
                detail: status_detail(&st),
                status: st,
            },
        ));
    }
    rows.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.label.cmp(&b.2.label))
    });
    rows.into_iter().map(|(_, _, r)| r).collect()
}

/// Summary counts and toolbar pill label for the board's live Activity tray.
pub fn activity_summary(rows: &[ActivityRow]) -> Option<ActivitySummary> {
    if rows.is_empty() {
        return None;
    }
    let mut running = 0;
    let mut needs_you = 0;
    let mut done = 0;
    let mut failed = 0;
    for r in rows {
        match r.status {
            BoxStatus::Working(_) => running += 1,
            BoxStatus::NeedsYou => needs_you += 1,
            BoxStatus::Done => done += 1,
            BoxStatus::Failed => failed += 1,
            BoxStatus::Archived | BoxStatus::CantSee => {}
        }
    }
    let label = if needs_you > 0 && running > 0 {
        format!("{needs_you} needs you · {running} running")
    } else if needs_you > 0 {
        format!("{needs_you} needs you")
    } else if running > 0 {
        format!("{running} running")
    } else if failed > 0 && done > 0 {
        format!("{done} done · {failed} failed")
    } else if failed > 0 {
        format!("{failed} failed")
    } else if done > 0 {
        format!("{done} done")
    } else {
        format!("{} sent", rows.len())
    };
    Some(ActivitySummary {
        running,
        needs_you,
        done,
        failed,
        label,
    })
}

// ---------------------------------------------------------------- Ask planner to review

/// The one message Ask planner to review sends, or None while the button
/// shouldn't show: no box sent, or any still going. Failed ones still let it
/// show, saying so. Nothing here sends anything: only Wayne's click does.
pub fn review_message(statuses: &BTreeMap<String, BoxStatus>, board: &Board) -> Option<String> {
    if statuses.is_empty() || statuses.values().any(|s| !s.settled()) {
        return None;
    }
    let failed: Vec<String> = statuses
        .iter()
        .filter(|(_, s)| !matches!(s, BoxStatus::Done))
        .map(|(id, _)| {
            board
                .graph
                .node(id)
                .map(|n| n.label.clone())
                .unwrap_or_else(|| id.clone())
        })
        .collect();
    if failed.is_empty() {
        Some("All sent boxes are Done. Please review the plan.".to_owned())
    } else {
        Some(format!(
            "All sent boxes have finished, but some failed or were archived ({}). Please review the plan.",
            failed.join(", ")
        ))
    }
}

// ---------------------------------------------------------------- accessibility

/// "Box KPI cards, in group Dashboard · React, Codex, needs you".
pub fn box_label(board: &Board, id: &str, status: Option<&BoxStatus>) -> String {
    let Some(n) = board.graph.node(id) else {
        return id.to_owned();
    };
    let mut parts = vec![format!("Box {}", n.label)];
    if let Some(g) = n.group.as_deref().and_then(|x| board.graph.group(x)) {
        parts.push(format!("in group {}", g.label));
    }
    if let Some(l) = board.links.get(id) {
        parts.push(if l.is_coordinator() {
            "project coordinator".to_owned()
        } else {
            match l.harness.as_deref() {
                Some("codex") => "Codex".to_owned(),
                Some(_) => "Claude".to_owned(),
                None => "sent".to_owned(),
            }
        });
    }
    if let Some(s) = status {
        parts.push(s.word().to_lowercase());
    }
    if board.changed.iter().any(|c| c == id) {
        parts.push("changed".to_owned());
    }
    parts.join(", ")
}

/// Boxes in plan order, for Tab.
pub fn tab_order(board: &Board) -> Vec<String> {
    board.graph.nodes.iter().map(|n| n.id.clone()).collect()
}

// ---------------------------------------------------------------- Phase 2: screens

/// A screen's still beside its box (planBoardMockups), as the runner lists it.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub frame: String,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub title: String,
    /// desktop, tablet or phone.
    #[serde(default)]
    pub viewport: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub changed: bool,
    #[serde(default)]
    pub picked: bool,
    /// The sealed file to draw, or a route on the thread's dev server.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
}

/// A viewport's size in CSS pixels.
pub fn viewport_size(viewport: &str) -> (f32, f32) {
    match viewport {
        "tablet" => (834., 1112.),
        "phone" => (390., 844.),
        _ => (1280., 800.),
    }
}

pub const STILL_W: f32 = 220.;

/// Where each of a box's frames goes: a strip beside the box (to its right
/// for a left-to-right plan, below it for top-to-bottom), one column per
/// variant (A, B, C), up to three.
pub fn frame_strip(node: &Rect, frames: &[Frame], tb: bool) -> Vec<(String, Rect)> {
    let mut variants: Vec<&str> = frames.iter().map(|f| f.variant.as_str()).collect();
    variants.sort();
    variants.dedup();
    variants.truncate(3);
    let mut out = Vec::new();
    let mut rows: HashMap<&str, f32> = HashMap::new();
    for f in frames {
        let Some(col) = variants.iter().position(|v| *v == f.variant) else {
            continue;
        };
        let (vw, vh) = viewport_size(&f.viewport);
        let h = STILL_W * vh / vw;
        let off = rows.entry(f.variant.as_str()).or_insert(0.);
        let (x, y) = if tb {
            (
                node.x + col as f32 * (STILL_W + 16.),
                node.bottom() + 24. + *off,
            )
        } else {
            (
                node.right() + 24. + col as f32 * (STILL_W + 16.),
                node.y + *off,
            )
        };
        *off += h + 30.;
        out.push((f.id.clone(), Rect::new(x, y, STILL_W, h)));
    }
    out
}

/// Pick B's chip: "▣ KPI cards · B", linked to the frame.
pub fn frame_chip(board: &Board, f: &Frame) -> String {
    let label = board
        .graph
        .node(&f.node)
        .map(|n| n.label.clone())
        .unwrap_or_else(|| f.node.clone());
    let name = format!("{label} · {}", f.variant)
        .replace('[', "\\[")
        .replace(']', "\\]");
    format!(
        "[{name}](board:{}#{}/{}@v{})",
        board.thread_id, f.node, f.id, f.version
    )
}

/// A region dragged on a still, in the frame's own CSS pixels.
pub fn region_css(still: &Rect, frame_viewport: &str, a: (f32, f32), b: (f32, f32)) -> [f32; 4] {
    let (vw, vh) = viewport_size(frame_viewport);
    let sx = vw / still.w.max(1.);
    let sy = vh / still.h.max(1.);
    let (x0, x1) = (a.0.min(b.0).max(still.x), a.0.max(b.0).min(still.right()));
    let (y0, y1) = (a.1.min(b.1).max(still.y), a.1.max(b.1).min(still.bottom()));
    [
        ((x0 - still.x) * sx).round(),
        ((y0 - still.y) * sy).round(),
        ((x1 - x0).max(0.) * sx).round(),
        ((y1 - y0).max(0.) * sy).round(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn acme() -> Board {
        Board::decode(&json!({
            "threadId": "5533c870-aaaa-bbbb-cccc-000000000001",
            "version": 1,
            "graph": {
                "direction": "LR",
                "nodes": [
                    { "id": "store", "label": "Storefront", "shape": "box" },
                    { "id": "orders", "label": "ORDERS", "shape": "db" },
                    { "id": "summary", "label": "GET /api/summary", "shape": "box", "group": "server" },
                    { "id": "kpi", "label": "KPI cards", "shape": "box", "group": "dash" },
                    { "id": "revenue", "label": "Revenue chart", "shape": "box", "group": "dash" },
                    { "id": "live", "label": "Live orders", "shape": "box", "group": "dash" }
                ],
                "groups": [ { "id": "server", "label": "SERVER · NODE" }, { "id": "dash", "label": "DASHBOARD · REACT" } ],
                "edges": [
                    { "from": "store", "to": "orders", "label": "new order" },
                    { "from": "orders", "to": "summary", "label": "read" },
                    { "from": "summary", "to": "kpi", "label": "every 5 s" },
                    { "from": "summary", "to": "revenue" },
                    { "from": "summary", "to": "live", "line": true }
                ],
                "notes": [], "skipped": 0
            },
            "note": { "text": "Orders reach the dashboard every 5 s", "near": "kpi" },
            "changed": ["kpi"],
            "approved": null,
            "positions": {},
            "skills": [],
            "links": {}
        }))
        .unwrap()
    }

    fn summary(f: impl FnOnce(&mut ThreadSummary)) -> ThreadSummary {
        let mut t = ThreadSummary {
            id: "t1".into(),
            status: "idle".into(),
            ..Default::default()
        };
        f(&mut t);
        t
    }

    #[test]
    fn the_next_step_line_follows_the_plan() {
        let mut b = acme();
        assert!(next_step(&b).contains("open Screens now"));
        assert!(next_step(&b).contains("Approve plan"));
        b.approved = Some(Approved {
            version: 1,
            at: String::new(),
        });
        assert!(next_step(&b).contains("press Send on any box"));
        b.version = 2;
        assert!(next_step(&b).contains("updated after approval"));
    }

    #[test]
    fn brief_with_screens_names_the_picked_take() {
        let mut b = acme();
        b.approved = Some(Approved {
            version: 1,
            at: String::new(),
        });
        let frames = vec![
            Frame {
                id: "kpi-row-A".into(),
                node: "kpi".into(),
                frame: "row".into(),
                variant: "A".into(),
                title: "Dense row".into(),
                viewport: "desktop".into(),
                version: 1,
                picked: false,
                path: Some("/tmp/a.html".into()),
                ..Default::default()
            },
            Frame {
                id: "kpi-row-B".into(),
                node: "kpi".into(),
                frame: "row".into(),
                variant: "B".into(),
                title: "Clean cards".into(),
                viewport: "desktop".into(),
                version: 2,
                picked: true,
                path: Some("/tmp/b.html".into()),
                ..Default::default()
            },
        ];
        let text = brief_with_screens(&b, "kpi", "claude", &[], &frames).unwrap();
        assert!(text
            .contains("Picked screen: \"Clean cards\" (variant B, desktop, v2): file /tmp/b.html"));
        assert!(!text.contains("Dense row"));
        let ask = ask_screens_message(&b, "kpi");
        assert!(ask.starts_with("[KPI cards](board:5533c870-aaaa-bbbb-cccc-000000000001#kpi)"));
    }

    #[test]
    fn layout_ranks_follow_the_arrows_left_to_right() {
        let b = acme();
        let l = layout(&b.graph, &b.positions);
        let x = |id: &str| l.rect(id).unwrap().x;
        assert!(x("store") < x("orders") && x("orders") < x("summary") && x("summary") < x("kpi"));
        let cx = |id: &str| l.rect(id).unwrap().center().0;
        assert_eq!(
            cx("kpi") as i32,
            cx("revenue") as i32,
            "one rank, one column"
        );
        // Same plan, same layout.
        assert_eq!(l, layout(&b.graph, &b.positions));
        // The group frames take in their boxes.
        let dash = l.group_rect("dash").unwrap();
        for id in ["kpi", "revenue", "live"] {
            let r = l.rect(id).unwrap();
            assert!(dash.contains((r.x, r.y)) && dash.contains((r.right(), r.bottom())));
        }
        assert_eq!(l.edges.len(), 5);
    }

    #[test]
    fn top_to_bottom_plans_rank_downwards() {
        let mut b = acme();
        b.graph.direction = "TB".into();
        let l = layout(&b.graph, &b.positions);
        assert!(l.rect("store").unwrap().y < l.rect("orders").unwrap().y);
    }

    #[test]
    fn loops_still_lay_out() {
        let mut b = acme();
        b.graph.edges.push(Edge {
            from: "kpi".into(),
            to: "store".into(),
            label: None,
            line: false,
        });
        let l = layout(&b.graph, &b.positions);
        assert!(l.rect("store").unwrap().x < l.rect("kpi").unwrap().x);
    }

    #[test]
    fn a_dragged_box_keeps_its_place_across_a_new_version_and_tidy_forgets_it() {
        let mut b = acme();
        b.positions.insert("kpi".into(), (900., -40.));
        let l = layout(&b.graph, &b.positions);
        assert_eq!(
            (l.rect("kpi").unwrap().x, l.rect("kpi").unwrap().y),
            (900., -40.)
        );
        // v2: a box added and labels changed; the moved one stays where it was.
        b.version = 2;
        b.graph.nodes.push(Node {
            id: "events".into(),
            label: "GET /events".into(),
            shape: "box".into(),
            group: Some("server".into()),
        });
        b.graph
            .nodes
            .iter_mut()
            .find(|n| n.id == "kpi")
            .unwrap()
            .label = "KPI tiles".into();
        let l2 = layout(&b.graph, &b.positions);
        assert_eq!(
            (l2.rect("kpi").unwrap().x, l2.rect("kpi").unwrap().y),
            (900., -40.)
        );
        // Tidy: no kept positions, so it goes back into its rank.
        let tidy = layout(&b.graph, &BTreeMap::new());
        assert_ne!(tidy.rect("kpi").unwrap().x, 900.);
    }

    #[test]
    fn hits_at_several_zooms() {
        let b = acme();
        let l = layout(&b.graph, &b.positions);
        let kpi = l.rect("kpi").unwrap();
        for s in [0.25, 1., 2.] {
            assert_eq!(hit(&l, kpi.center(), s), Some(Hit::Box("kpi".into())));
            let mid = l.edges[2].mid();
            assert_eq!(hit(&l, mid, s), Some(Hit::Arrow(2)), "zoom {s}");
        }
        let dash = l.group_rect("dash").unwrap();
        assert_eq!(
            hit(&l, (dash.x + 4., dash.y + 4.), 1.),
            Some(Hit::Group("dash".into()))
        );
        assert_eq!(hit(&l, (-5000., -5000.), 1.), None);
    }

    #[test]
    fn a_selection_becomes_a_chip_the_agent_can_read() {
        let b = acme();
        assert_eq!(
            chip_markdown(&b, &Hit::Box("kpi".into())),
            "[KPI cards](board:5533c870-aaaa-bbbb-cccc-000000000001#kpi)"
        );
        assert_eq!(
            chip_markdown(&b, &Hit::Group("server".into())),
            "[SERVER · NODE](board:5533c870-aaaa-bbbb-cccc-000000000001#server)"
        );
        assert_eq!(
            chip_markdown(&b, &Hit::Arrow(2)),
            "[GET /api/summary → KPI cards (every 5 s)](board:5533c870-aaaa-bbbb-cccc-000000000001#summary->kpi)"
        );
        assert_eq!(parse_board_link("board:abc#kpi"), Some(("abc", "kpi")));
        assert_eq!(parse_board_link("thread:abc"), None);
    }

    #[test]
    fn plan_state_and_every_box_greying_after_a_change() {
        let mut b = acme();
        assert_eq!(b.state().words(), "Draft v1");
        assert_eq!(
            send_allowed(&b, "revenue", None),
            Err("Approve the plan first")
        );
        b.approved = Some(Approved {
            version: 1,
            at: String::new(),
        });
        assert_eq!(b.state().words(), "Approved v1");
        assert_eq!(send_allowed(&b, "revenue", None), Ok(()));
        b.version = 2;
        assert_eq!(b.state().words(), "Changed since you approved (v1 → v2)");
        for n in tab_order(&b) {
            assert!(send_allowed(&b, &n, None).is_err(), "{n} greys too");
        }
        // A box already sent and still going can't be sent again; a failed one can once approved.
        b.approved = Some(Approved {
            version: 2,
            at: String::new(),
        });
        assert_eq!(
            send_allowed(&b, "kpi", Some(BoxStatus::Working(String::new()))),
            Err("Already sent")
        );
        assert_eq!(send_allowed(&b, "kpi", Some(BoxStatus::Failed)), Ok(()));
    }

    #[test]
    fn the_brief_has_the_header_then_skills_in_each_form() {
        let mut b = acme();
        b.approved = Some(Approved {
            version: 1,
            at: String::new(),
        });
        let skills = vec!["component-canvas".to_owned(), "shadcn".to_owned()];
        let claude = brief(&b, "kpi", "claude", &skills).unwrap();
        let lines: Vec<&str> = claude.lines().collect();
        assert_eq!(
            lines[0],
            "Plan board: thread 5533c870, box kpi (\"KPI cards\"), plan v1 (approved)."
        );
        assert_eq!(lines[1], "Use these skills: /component-canvas, /shadcn");
        assert!(claude.contains("GET /api/summary (every 5 s) → this"));
        assert!(claude.contains("in the group \"DASHBOARD · REACT\""));
        assert!(claude.contains("Planner's note: Orders reach the dashboard every 5 s"));
        let codex = brief(&b, "kpi", "codex", &skills).unwrap();
        assert_eq!(
            codex.lines().nth(1),
            Some("Use these skills: $component-canvas, $shadcn")
        );
        let plain = brief(&b, "kpi", "claude", &[]).unwrap();
        assert_eq!(plain.lines().nth(1), Some(""));
        let (ok, missing) = skills_for_login(&skills, &["shadcn".to_owned()]);
        assert_eq!(
            (ok, missing),
            (
                vec!["shadcn".to_owned()],
                vec!["component-canvas".to_owned()]
            )
        );
    }

    #[test]
    fn status_follows_the_thread_through_the_inbox_rules() {
        assert_eq!(status(None), BoxStatus::CantSee);
        assert_eq!(
            status(Some(&summary(|t| t.status = "running".into()))),
            BoxStatus::Working(String::new())
        );
        assert_eq!(
            status(Some(&summary(|t| {
                t.status = "running".into();
                t.preview = "Edit src/components/KpiCard.tsx".into();
            }))),
            BoxStatus::Working("Edit src/components/KpiCard.tsx".into())
        );
        assert_eq!(
            status(Some(&summary(|t| t.status = "waiting".into()))),
            BoxStatus::NeedsYou,
            "a card: on_card"
        );
        assert_eq!(
            status(Some(&summary(|t| t.needs = Some("you".into())))),
            BoxStatus::NeedsYou,
            "waiting_on_mike"
        );
        assert_eq!(
            status(Some(&summary(|t| t.needs = Some("nothing".into())))),
            BoxStatus::Done
        );
        assert_eq!(
            status(Some(&summary(|t| t.status = "error".into()))),
            BoxStatus::Failed
        );
        assert_eq!(
            status(Some(&summary(
                |t| t.archived_at = Some("2026-11-26T00:00:00Z".into())
            ))),
            BoxStatus::Archived
        );
    }

    #[test]
    fn a_board_sent_thread_that_comes_to_need_you_notifies_like_any_other() {
        // Nothing board-only: the inbox's own should_notify fires for it.
        let before = summary(|t| t.status = "running".into());
        let after = summary(|t| {
            t.needs_you = true;
            t.needs = Some("you".into());
            t.started_by =
                Some(json!({ "kind": "thread", "label": "Plan board", "threadId": "planner" }));
        });
        assert!(crate::inbox::should_notify(Some(&before), &after, false));
        assert!(
            !crate::inbox::should_notify(Some(&before), &after, true),
            "not while looking at it"
        );
        assert_eq!(status(Some(&after)), BoxStatus::NeedsYou);
    }

    #[test]
    fn ask_planner_to_review_only_once_every_sent_box_has_finished() {
        let b = acme();
        let mut s = BTreeMap::new();
        assert_eq!(review_message(&s, &b), None, "nothing sent");
        s.insert("kpi".to_owned(), BoxStatus::Done);
        s.insert("revenue".to_owned(), BoxStatus::Working(String::new()));
        assert_eq!(
            review_message(&s, &b),
            None,
            "one still running: a Done box wakes nothing"
        );
        s.insert("revenue".to_owned(), BoxStatus::Done);
        assert_eq!(
            review_message(&s, &b).as_deref(),
            Some("All sent boxes are Done. Please review the plan.")
        );
        s.insert("revenue".to_owned(), BoxStatus::Failed);
        assert!(review_message(&s, &b)
            .unwrap()
            .contains("some failed or were archived (Revenue chart)"));
        s.insert("live".to_owned(), BoxStatus::NeedsYou);
        assert_eq!(review_message(&s, &b), None);
    }

    #[test]
    fn coordinator_tasks_map_onto_the_same_badges() {
        let coord = summary(|_| {});
        let task = |state: &str| CoordinatorTask {
            id: "t1".into(),
            state: state.into(),
            ..Default::default()
        };
        assert_eq!(coordinator_status(None, Some(&coord)), BoxStatus::CantSee);
        assert_eq!(
            coordinator_status(Some(&task("todo")), None),
            BoxStatus::CantSee,
            "coordinator gone"
        );
        assert!(matches!(
            coordinator_status(Some(&task("todo")), Some(&coord)),
            BoxStatus::Working(_)
        ));
        assert!(matches!(
            coordinator_status(Some(&task("working")), Some(&coord)),
            BoxStatus::Working(_)
        ));
        assert!(
            matches!(
                coordinator_status(Some(&task("blocked")), Some(&coord)),
                BoxStatus::Working(_)
            ),
            "handling a helper's question"
        );
        let asked = CoordinatorTask {
            asked_wayne: true,
            ..task("blocked")
        };
        assert_eq!(
            coordinator_status(Some(&asked), Some(&coord)),
            BoxStatus::NeedsYou
        );
        assert!(matches!(
            coordinator_status(Some(&task("review")), Some(&coord)),
            BoxStatus::Working(_)
        ));
        assert_eq!(
            coordinator_status(Some(&task("done")), Some(&coord)),
            BoxStatus::Done
        );
        assert_eq!(
            coordinator_status(
                Some(&CoordinatorTask {
                    failed: true,
                    ..task("working")
                }),
                Some(&coord)
            ),
            BoxStatus::Failed
        );
        assert_eq!(
            coordinator_status(
                Some(&CoordinatorTask {
                    archived: true,
                    ..task("done")
                }),
                Some(&coord)
            ),
            BoxStatus::Archived
        );
    }

    #[test]
    fn labels_name_the_box_its_group_harness_and_status() {
        let mut b = acme();
        b.links.insert(
            "kpi".into(),
            Link {
                thread_id: Some("a1".into()),
                harness: Some("codex".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            box_label(&b, "kpi", Some(&BoxStatus::NeedsYou)),
            "Box KPI cards, in group DASHBOARD · REACT, Codex, needs you, changed"
        );
    }

    #[test]
    fn a_new_version_is_one_linked_note_in_the_transcript() {
        use crate::reducer::{Event, RowContent, TranscriptModel};
        let ev = Event::decode(&json!({
            "seq": 4, "ts": "2026-11-26T00:00:00Z", "threadId": "5533c870-aa", "type": "board.updated",
            "board": { "threadId": "5533c870-aa", "version": 2, "graph": { "nodes": [] } }
        }))
        .unwrap();
        let model = TranscriptModel::from_events(vec![ev], None);
        let row = model
            .rows()
            .iter()
            .find(|r| r.id == "board:4")
            .expect("a row");
        assert!(
            matches!(&row.content, RowContent::Note(t) if t == "Drew a plan (v2) · [Show plan](board:5533c870-aa)")
        );
        assert!(row.content.is_linked_note());
    }

    #[test]
    fn screens_sit_beside_their_box_in_variant_columns() {
        let node = Rect::new(0., 0., 150., BOX_H);
        let f = |id: &str, v: &str| Frame {
            id: id.into(),
            node: "kpi".into(),
            variant: v.into(),
            viewport: "desktop".into(),
            version: 1,
            ..Default::default()
        };
        let strip = frame_strip(
            &node,
            &[f("a", "A"), f("b", "B"), f("c", "C"), f("d", "D")],
            false,
        );
        assert_eq!(strip.len(), 3, "three variants at most");
        assert!(strip[0].1.x > node.right());
        assert!(strip[1].1.x > strip[0].1.right());
        assert_eq!(strip[0].1.h, STILL_W * 800. / 1280.);
        let b = acme();
        assert_eq!(
            frame_chip(&b, &f("b", "B")),
            "[KPI cards · B](board:5533c870-aaaa-bbbb-cccc-000000000001#kpi/b@v1)"
        );
    }

    #[test]
    fn a_region_on_a_still_is_in_the_frames_css_pixels_at_any_zoom() {
        let still = Rect::new(100., 100., 220., 137.5);
        assert_eq!(
            region_css(&still, "desktop", (100., 100.), (210., 168.75)),
            [0., 0., 640., 400.]
        );
        // The same region measured on a still drawn twice as big.
        let big = Rect::new(100., 100., 440., 275.);
        assert_eq!(
            region_css(&big, "desktop", (100., 100.), (320., 237.5)),
            [0., 0., 640., 400.]
        );
        // Clamped to the still.
        assert_eq!(
            region_css(&still, "phone", (0., 0.), (1000., 1000.)),
            [0., 0., 390., 844.]
        );
    }

    #[test]
    fn activity_rows_rank_needs_you_and_running_ahead_of_done_and_include_planner() {
        let mut b = acme();
        b.links.insert(
            "store".into(),
            Link {
                kind: Some("thread".into()),
                thread_id: Some("t-store".into()),
                harness: Some("claude".into()),
                ..Default::default()
            },
        );
        b.links.insert(
            "kpi".into(),
            Link {
                kind: Some("thread".into()),
                thread_id: Some("t-kpi".into()),
                harness: Some("codex".into()),
                ..Default::default()
            },
        );
        b.links.insert(
            "revenue".into(),
            Link {
                kind: Some("coordinator".into()),
                thread_id: Some("t-coord".into()),
                task_id: Some("task-1".into()),
                ..Default::default()
            },
        );

        let mut sts = BTreeMap::new();
        sts.insert("store".into(), BoxStatus::Done);
        sts.insert("kpi".into(), BoxStatus::NeedsYou);
        sts.insert("revenue".into(), BoxStatus::Working("Running tests".into()));

        let planner = summary(|t| {
            t.id = "t-plan".into();
            t.title = "Plan".into();
            t.status = "idle".into();
            t.provider = "claude".into();
        });
        let lookup = |id: &str| {
            Some(summary(|t| {
                t.id = id.into();
                t.title = format!("Thread {id}");
            }))
        };

        let rows = activity_rows(
            &b,
            Some(&planner),
            Some("Generating 3 screen takes for KPI cards"),
            &sts,
            lookup,
        );
        // Order: NeedsYou (kpi) -> Working (Planner, then revenue) -> Done (store)
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].node_id.as_deref(), Some("kpi"));
        assert_eq!(rows[0].where_name, "Codex");
        assert_eq!(rows[0].status, BoxStatus::NeedsYou);
        assert_eq!(rows[0].thread_title, "Thread t-kpi");

        assert_eq!(rows[1].node_id, None);
        assert_eq!(rows[1].label, "Planner");
        assert_eq!(rows[1].where_name, "Claude");
        assert_eq!(rows[1].detail, "Generating 3 screen takes for KPI cards");

        assert_eq!(rows[2].node_id.as_deref(), Some("revenue"));
        assert_eq!(rows[2].where_name, "Coordinator");
        assert_eq!(rows[2].detail, "Running tests");

        assert_eq!(rows[3].node_id.as_deref(), Some("store"));
        assert_eq!(rows[3].status, BoxStatus::Done);

        let sum = activity_summary(&rows).expect("summary");
        assert_eq!(sum.needs_you, 1);
        assert_eq!(sum.running, 2);
        assert_eq!(sum.done, 1);
        assert_eq!(sum.failed, 0);
        assert_eq!(sum.label, "1 needs you · 2 running");
    }

    #[test]
    fn activity_summary_is_none_when_nothing_is_sent_or_running() {
        let b = acme();
        let planner = summary(|t| t.status = "idle".into());
        let rows = activity_rows(&b, Some(&planner), None, &BTreeMap::new(), |_| None);
        assert!(rows.is_empty());
        assert_eq!(activity_summary(&rows), None);

        // Even if the planner thread has needs="you" (shown on the main thread's
        // own composer card), the board's Activity pill does not duplicate it.
        let waiting_planner = summary(|t| {
            t.status = "waiting".into();
            t.needs = Some("you".into());
        });
        let rows_waiting =
            activity_rows(&b, Some(&waiting_planner), None, &BTreeMap::new(), |_| None);
        assert!(rows_waiting.is_empty());
    }
}
