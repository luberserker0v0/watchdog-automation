use std::collections::{HashMap, HashSet, VecDeque};

use crate::modules::pipeline::authoring::models::graph::PipelineGraph;
use crate::modules::pipeline::authoring::models::node::NodeKind;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub node_id: String,
    pub hops: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTiers {
    pub local: Vec<Candidate>,
    pub expanded: Vec<Candidate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateScore {
    pub node_id: String,
    pub score: f64,
}

/// A VLM response is advisory until every candidate is scored with a finite
/// value in [0, 1]. Ambiguous results produce an inconclusive report.
pub fn select_confident_match(
    candidates: &[Candidate],
    scores: &[CandidateScore],
    threshold: f64,
    min_margin: f64,
) -> Result<Option<String>, String> {
    select_match(candidates, scores, threshold, min_margin, None)
}

/// A positive, non-trivial recognizer result is independent evidence for the
/// anchor. GUI states can overlap: a title, account field, and password field
/// may all be visible in one frame. Accept the anchor when it clears the
/// absolute threshold and no other candidate scores higher. Possible mismatch
/// reports still require the configured margin. A timed-out recognizer does
/// not supply this preference.
pub(super) fn select_match(
    candidates: &[Candidate],
    scores: &[CandidateScore],
    threshold: f64,
    min_margin: f64,
    confirmed_anchor: Option<&str>,
) -> Result<Option<String>, String> {
    if candidates.is_empty() {
        return Ok(None);
    }
    let expected: HashSet<&str> = candidates
        .iter()
        .map(|candidate| candidate.node_id.as_str())
        .collect();
    let mut observed = HashSet::new();
    for result in scores {
        if !expected.contains(result.node_id.as_str()) || !observed.insert(result.node_id.as_str())
        {
            return Err(format!(
                "Patrol VLM returned an unknown or repeated node '{}'; report inconclusive",
                result.node_id
            ));
        }
        if !result.score.is_finite() || !(0.0..=1.0).contains(&result.score) {
            return Err(format!(
                "Patrol VLM returned an invalid score for '{}'; report inconclusive",
                result.node_id
            ));
        }
    }
    if observed.len() != expected.len() {
        return Err("Patrol VLM did not score every candidate; report inconclusive".into());
    }
    if let Some(anchor) = confirmed_anchor {
        if let Some(anchor_score) = scores.iter().find(|score| score.node_id == anchor) {
            if anchor_score.score >= threshold
                && scores.iter().all(|score| score.score <= anchor_score.score)
            {
                return Ok(Some(anchor.to_string()));
            }
        }
    }
    let mut ranked = scores.to_vec();
    ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
    let best = &ranked[0];
    if best.score < threshold
        || ranked
            .get(1)
            .is_some_and(|second| best.score - second.score < min_margin)
    {
        return Ok(None);
    }
    Ok(Some(best.node_id.clone()))
}

pub(super) fn inconclusive_scores(
    scores: &[CandidateScore],
    threshold: f64,
    min_margin: f64,
) -> String {
    let mut ranked = scores.to_vec();
    ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
    let leaders = ranked
        .iter()
        .take(2)
        .map(|score| format!("{}={:.2}", score.node_id, score.score))
        .collect::<Vec<_>>()
        .join(", ");
    format!("top scores: {leaders}; threshold={threshold:.2}, required gap={min_margin:.2}")
}

/// The first tier includes the anchor and its K-hop neighborhood. The second
/// tier adds nodes through 2K hops, including siblings connected by a shared
/// parent. Entry nodes have no visual state and are excluded.
pub fn candidate_tiers(graph: &PipelineGraph, anchor: &str, k_hops: usize) -> CandidateTiers {
    let max_hops = k_hops.saturating_mul(2);
    let known: HashSet<&str> = graph.nodes.iter().map(|node| node.id.as_str()).collect();
    if !known.contains(anchor) {
        return CandidateTiers {
            local: Vec::new(),
            expanded: Vec::new(),
        };
    }

    let mut adjacent: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        if known.contains(edge.from.as_str()) && known.contains(edge.to.as_str()) {
            adjacent.entry(&edge.from).or_default().push(&edge.to);
            adjacent.entry(&edge.to).or_default().push(&edge.from);
        }
    }
    let mut visited = HashSet::from([anchor]);
    let mut queue = VecDeque::from([(anchor, 0usize)]);
    let mut local = Vec::new();
    let mut expanded = Vec::new();

    while let Some((id, hops)) = queue.pop_front() {
        if graph
            .find_node(id)
            .is_some_and(|node| node.kind == NodeKind::Normal)
        {
            let candidate = Candidate {
                node_id: id.to_string(),
                hops,
            };
            if hops <= k_hops {
                local.push(candidate);
            } else {
                expanded.push(candidate);
            }
        }
        if hops >= max_hops {
            continue;
        }
        if let Some(neighbors) = adjacent.get(id) {
            for neighbor in neighbors {
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, hops + 1));
                }
            }
        }
    }
    CandidateTiers { local, expanded }
}

/// Keep VLM requests bounded while retaining the route-critical states first.
/// Candidate order is stable across runs because graph edge and node order are
/// used after the explicit priorities.
pub fn bounded_candidates(
    graph: &PipelineGraph,
    anchor: &str,
    candidates: Vec<Candidate>,
    recent_nodes: &[String],
    limit: usize,
) -> Vec<Candidate> {
    let by_id = candidates
        .into_iter()
        .map(|candidate| (candidate.node_id.clone(), candidate))
        .collect::<HashMap<_, _>>();
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |node_id: &str| {
        if ordered.len() < limit && seen.insert(node_id.to_string()) {
            if let Some(candidate) = by_id.get(node_id) {
                ordered.push(candidate.clone());
            }
        }
    };
    push(anchor);
    for edge in graph.next_edges(anchor) {
        push(&edge.to);
    }
    for edge in graph.error_edges(anchor) {
        push(&edge.to);
    }
    for node_id in recent_nodes.iter().rev() {
        push(node_id);
    }
    let graph_order = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut remaining = by_id.values().collect::<Vec<_>>();
    remaining.sort_by_key(|candidate| {
        (
            candidate.hops,
            graph_order
                .get(candidate.node_id.as_str())
                .copied()
                .unwrap_or(usize::MAX),
        )
    });
    for candidate in remaining {
        push(&candidate.node_id);
    }
    ordered
}

pub fn recovery_candidates(
    graph: &PipelineGraph,
    anchor: &str,
    recent_nodes: &[String],
    k_hops: usize,
) -> Vec<Candidate> {
    let known = graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    if !known.contains(anchor) {
        return Vec::new();
    }
    let mut adjacent: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        if known.contains(edge.from.as_str()) && known.contains(edge.to.as_str()) {
            adjacent.entry(&edge.from).or_default().push(&edge.to);
            adjacent.entry(&edge.to).or_default().push(&edge.from);
        }
    }
    let mut distances = HashMap::from([(anchor, 0usize)]);
    let mut queue = VecDeque::from([anchor]);
    while let Some(node_id) = queue.pop_front() {
        let distance = distances[node_id];
        for neighbor in adjacent.get(node_id).into_iter().flatten() {
            if !distances.contains_key(neighbor) {
                distances.insert(neighbor, distance + 1);
                queue.push_back(neighbor);
            }
        }
    }
    let eligible = |node_id: &str| {
        graph
            .find_node(node_id)
            .is_some_and(|node| node.kind == NodeKind::Normal && node.enabled)
    };
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |node_id: &str| {
        if result.len() < 16 && seen.insert(node_id.to_string()) && eligible(node_id) {
            result.push(Candidate {
                node_id: node_id.to_string(),
                hops: distances.get(node_id).copied().unwrap_or(usize::MAX),
            });
        }
    };
    push(anchor);
    let mut local = graph
        .nodes
        .iter()
        .filter_map(|node| {
            distances
                .get(node.id.as_str())
                .copied()
                .filter(|hops| *hops > 0 && *hops <= k_hops)
                .map(|hops| (node.id.as_str(), hops))
        })
        .collect::<Vec<_>>();
    local.sort_by_key(|(_, hops)| *hops);
    for (node_id, _) in local {
        push(node_id);
    }
    for node_id in recent_nodes.iter().rev() {
        push(node_id);
    }
    let mut remaining = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            (
                node.id.as_str(),
                distances
                    .get(node.id.as_str())
                    .copied()
                    .unwrap_or(usize::MAX),
                index,
            )
        })
        .collect::<Vec<_>>();
    remaining.sort_by_key(|(_, hops, index)| (*hops, *index));
    for (node_id, _, _) in remaining {
        push(node_id);
    }
    result
}
