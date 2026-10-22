//! Checks the spec's performance targets on a synthetic 5,000-note vault.
//! Run with `cargo test --release -p stash-core --test perf -- --ignored --nocapture`.

use stash_core::index::SearchOptions;
use stash_core::note::TicketMatcher;
use stash_core::{dates, Index, Vault};
use std::collections::HashMap;
use std::time::Instant;

const WORDS: &[&str] = &[
    "rollout", "deploy", "review", "gerrit", "build", "agent", "flaky", "test", "meeting", "customer", "latency",
    "cache", "config", "migration", "database", "release", "branch", "merge", "incident", "alert",
];

fn word(i: usize) -> &'static str {
    WORDS[(i * 7 + i / 3) % WORDS.len()]
}

#[test]
#[ignore]
fn five_thousand_notes() {
    let dir = tempfile::tempdir().unwrap();
    let vault = Vault::new(dir.path());
    for i in 0..5000 {
        let body: String = (0..30)
            .map(|l| format!("- 2026-10-{:02} 1{}:0{} {} {} PROJ-{} {}\n", 1 + l % 28, l % 10, l % 10, word(i + l), word(i * 3 + l), (i + l) % 900, word(l)))
            .collect();
        let (path, text) = match i % 3 {
            0 => (format!("tickets/PROJ-{i}.md"), format!("---\ntitle: {} {}\n---\n{body}", word(i), word(i + 1))),
            1 => (format!("notes/note-{i}.md"), format!("# {} {} {i}\n{body}", word(i), word(i + 2))),
            _ => (format!("daily/{}.md", dates::parse_iso("2015-01-01").unwrap() + chrono::Duration::days(i as i64)), body),
        };
        vault.write(&path, &text).unwrap();
    }

    let t = Instant::now();
    let notes = vault.scan();
    let mut index = Index::new(TicketMatcher::default_pattern());
    for (p, c, m) in &notes {
        index.upsert(p, c, *m);
    }
    let build = t.elapsed();
    println!("index build: {} notes in {build:?}", index.len());
    assert_eq!(index.len(), 5000);

    let recents = HashMap::new();
    let opts = SearchOptions {
        prefer: None,
        include_archived: false,
        limit: 50,
        current: None,
        today: dates::parse_iso("2026-10-22").unwrap(),
        now_ms: 0,
        recents: &recents,
        statuses: &stash_core::status::default_statuses(),
    };
    let mut worst = std::time::Duration::ZERO;
    for q in ["1", "12", "123", "r", "ro", "rol", "roll", "rollout", "rlt", "proj-12", "yesterday", "oct 3", "flaky test"] {
        let t = Instant::now();
        let res = index.search(q, &opts);
        let d = t.elapsed();
        worst = worst.max(d);
        println!("search {q:>10}: {:>3} hits in {d:?}", res.hits.len());
    }
    println!("worst keystroke: {worst:?}");
    if !cfg!(debug_assertions) {
        assert!(build.as_millis() < 1000, "index build too slow: {build:?}");
        assert!(worst.as_millis() < 16, "search too slow: {worst:?}");
    }
}
