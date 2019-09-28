mod author;
mod file;
mod options;
mod plotting;
mod repo;
mod stat;

use options::ConstatOptions;
use plotting::Renderer;
use stat::PendingStat;

use chrono::{Duration, TimeZone, Utc};
use plotters::prelude::*;

use std::collections::{HashMap, HashSet};

fn main() {
    let options = ConstatOptions::new();

    let mut ps = PendingStat::new(&options.repo_path, &options.patterns[..]);

    if let Some(since) = options.since {
        ps.base_commit(since);
    }

    let mut author_info: HashMap<_, Vec<(_, usize)>> = HashMap::new();

    let mut pb = None;

    let quiet = options.quiet;

    ps.run(|commit, stat, authors, total| {
        if !quiet {
            if pb.is_none() {
                pb = Some(indicatif::ProgressBar::new(total as u64));
            }

            pb.as_ref().unwrap().inc(1);
        }
        for (i, s) in (0..).zip(stat.iter()) {
            let value = author_info
                .entry(authors.get_name_by_id(i).unwrap_or("N/A").to_string())
                .or_insert_with(|| vec![]);

            let timestamp = Utc.ymd(1970, 1, 1) + Duration::seconds(commit.time().seconds());
            if let Some(what) = value.last_mut() {
                if what.0 == timestamp {
                    what.1 = what.1.max(*s);
                    continue;
                }
            }

            value.push((timestamp, *s));
        }
    })
    .unwrap();

    let author_info = {
        let exclude_older = options.exclude_older;
        let mut max_loc: Vec<_> = author_info
            .iter()
            .filter_map(|(name, stat)| {
                if exclude_older && name == "Older Code" {
                    None
                } else {
                    Some((name.to_string(), stat.iter().map(|x| x.1).max().unwrap()))
                }
            })
            .collect();

        max_loc.sort_by_key(|x| std::cmp::Reverse(x.1));

        max_loc.truncate(options.top);

        let mut others = HashMap::new();

        let is_top_authors: HashSet<_> = max_loc.iter().map(|(name, _)| name.as_ref()).collect();

        for (name, stats) in author_info.iter() {
            if is_top_authors.contains(&name[..]) || (exclude_older && name == "Older Code") {
                continue;
            }

            for (t, c) in stats {
                *others.entry(t.clone()).or_insert(0) += c;
            }
        }

        let mut others: Vec<_> = others.into_iter().collect();
        others.sort();

        let mut buf = vec![];

        for (name, _) in max_loc {
            let mut stat = author_info.remove(&name).unwrap();
            stat.sort();
            buf.push((name.to_string(), stat));
        }

        if !others.is_empty() && !options.top_only {
            buf.push(("Others".to_string(), others));
        }

        buf.sort_by_key(|(_name, stats)| {
            /*if name == "Older Code" {
                Utc.ymd(1969, 1, 1)
            } else if name == "Others" {
                Utc.ymd(1970, 1, 1)
            } else*/
            {
                stats.first().unwrap().0
            }
        });

        buf
    };

    if options
        .out_path
        .extension()
        .map_or(true, |ext| ext == "svg")
    {
        let renderer = Renderer::new(
            options.repo_path,
            author_info,
            SVGBackend::new(&options.out_path, options.resolution),
        );

        renderer.draw();
    } else {
        let renderer = Renderer::new(
            options.repo_path,
            author_info,
            BitMapBackend::new(&options.out_path, options.resolution),
        );

        renderer.draw();
    }

    if options.open {
        open::that(options.out_path).ok();
    }
}
