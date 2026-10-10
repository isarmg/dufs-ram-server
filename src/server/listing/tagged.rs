use super::*;
use crate::server::tagging::db::{FileQuery, ListedTags, Sample};

#[derive(Default, Serialize)]
pub(super) struct TagFilter {
    all: Vec<i64>,
    any: Vec<i64>,
    exclude: Vec<i64>,
}

impl TagFilter {
    pub(super) fn parse(params: &HashMap<String, String>) -> Result<Self> {
        fn ids(value: Option<&String>) -> Result<Vec<i64>> {
            let Some(value) = value.filter(|value| !value.is_empty()) else {
                return Ok(Vec::new());
            };
            anyhow::ensure!(value.len() <= 2100, "invalid tag filter");
            let mut ids = value
                .split(',')
                .map(str::parse::<i64>)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            anyhow::ensure!(
                ids.len() <= 100 && ids.iter().all(|id| *id > 0),
                "invalid tag filter"
            );
            ids.sort_unstable();
            ids.dedup();
            Ok(ids)
        }
        Ok(Self {
            all: ids(params.get("all"))?,
            any: ids(params.get("any"))?,
            exclude: ids(params.get("exclude"))?,
        })
    }
    pub(super) fn active(&self) -> bool {
        !self.all.is_empty() || !self.any.is_empty() || !self.exclude.is_empty()
    }
}

fn sample(rooted: &RootedFs, path: &Path, root: &Path) -> Option<Sample> {
    let metadata = rooted.metadata_nofollow_blocking(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let relative = path.strip_prefix(root).ok()?;
    Some(Sample {
        path: relative.to_str()?.into(),
        parent: relative.parent()?.to_str()?.into(),
        name: relative.file_name()?.to_str()?.into(),
        dev: metadata.dev() as i64,
        ino: metadata.ino() as i64,
        size: i64::try_from(metadata.len()).ok()?,
        mtime_ns: metadata
            .mtime()
            .checked_mul(1_000_000_000)?
            .checked_add(metadata.mtime_nsec())?,
        ctime_ns: metadata
            .ctime()
            .checked_mul(1_000_000_000)?
            .checked_add(metadata.ctime_nsec())?,
    })
}

impl Server {
    pub(super) async fn filter_tagged_paths(
        &self,
        path: &Path,
        paths: Vec<PathItem>,
        filter: TagFilter,
        recursive: bool,
    ) -> Result<Vec<PathItem>> {
        let db = self.tagging.db.clone();
        let permit = db.admission.clone().try_acquire_owned()?;
        let rooted = self.content.rooted_fs.clone();
        let root = self.content.args.serve_path.clone();
        let base = path.to_owned();
        let include_untagged = filter.all.is_empty() && filter.any.is_empty();
        let cancellation = CancellationToken::new();
        let _cancel_on_drop = CancelOnDrop::new(cancellation.clone());
        let running = self.lifecycle.running.clone();
        let max_duration = Duration::from_secs(self.content.args.request_timeout);
        let query = FileQuery {
            directory: path
                .strip_prefix(&root)?
                .to_str()
                .ok_or_else(|| anyhow!("invalid directory"))?
                .into(),
            search: String::new(),
            scope: if recursive { "recursive" } else { "current" }.into(),
            status: "all".into(),
            all_tags: filter.all,
            any_tags: filter.any,
            exclude_tags: filter.exclude,
            page: 1,
            page_size: 100,
        };
        spawn_directory_blocking(&self.lifecycle.work_tasks, None, move || {
            let _permit = permit;
            let started = Instant::now();
            let matches = db.matching_tag_paths(&query)?;
            let mut selected = Vec::new();
            selected.try_reserve(paths.len())?;
            for item in paths {
                anyhow::ensure!(
                    !cancellation.is_cancelled()
                        && running.load(atomic::Ordering::Relaxed)
                        && started.elapsed() < max_duration,
                    "tag filter cancelled or timed out"
                );
                if item.path_type == PathType::File
                    && sample(&rooted, &base.join(&item.name), &root).is_some_and(|sample| {
                        match matches.get(&sample.path) {
                            Some((identity, selected)) if *identity == sample.identity() => {
                                *selected
                            }
                            _ => include_untagged,
                        }
                    })
                {
                    selected.push(item);
                }
            }
            Ok(selected)
        })
        .await
        .map_err(std::io::Error::other)?
    }

    async fn tags_for_entries(
        &self,
        path: &Path,
        names: Vec<Option<String>>,
    ) -> Result<Vec<ListedTags>> {
        let db = self.tagging.db.clone();
        let permit = db.admission.clone().try_acquire_owned()?;
        let rooted = self.content.rooted_fs.clone();
        let root = self.content.args.serve_path.clone();
        let base = path.to_owned();
        spawn_directory_blocking(&self.lifecycle.work_tasks, None, move || {
            let _permit = permit;
            let samples: Vec<_> = names
                .into_iter()
                .map(|name| name.and_then(|name| sample(&rooted, &base.join(name), &root)))
                .collect();
            // A scan or another short read can own the single SQLite connection.
            let until = Instant::now() + Duration::from_millis(500);
            loop {
                match db.listed_tags(&samples) {
                    Err(error)
                        if error
                            .downcast_ref::<crate::server::tagging::db::Busy>()
                            .is_some()
                            && Instant::now() < until =>
                    {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    result => return result,
                }
            }
        })
        .await
        .map_err(std::io::Error::other)?
    }

    pub(in crate::server) async fn tags_for_file(&self, relative: &str) -> Result<ListedTags> {
        let root = self.content.args.serve_path.clone();
        let mut result = self
            .tags_for_entries(&root, vec![Some(relative.into())])
            .await?;
        Ok(result.pop().unwrap_or_default())
    }

    pub(super) async fn send_tagged_list_page(
        &self,
        res: &mut Response,
        page: ListSnapshotPage,
        path: &Path,
    ) -> Result<()> {
        let names = page
            .paths()
            .iter()
            .map(|item| (item.path_type == PathType::File).then(|| item.name.clone()))
            .collect();
        let tags = match self.tags_for_entries(path, names).await {
            Ok(tags) => Some(tags),
            Err(_) => {
                log::warn!("Visible file tags are temporarily unavailable");
                None
            }
        };
        write_list_response(res, page, tags)
    }
}
