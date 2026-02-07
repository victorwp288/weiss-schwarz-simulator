use anyhow::Result;
use rayon::{ThreadPool, ThreadPoolBuilder};

pub(super) fn build_thread_pool(
    num_threads: Option<usize>,
    num_envs: usize,
) -> Result<(Option<ThreadPool>, Option<usize>)> {
    let Some(threads) = num_threads else {
        return Ok((None, None));
    };
    if threads == 0 {
        anyhow::bail!("num_threads must be > 0");
    }
    let capped = threads.min(num_envs.max(1));
    if capped > 1 {
        let pool = ThreadPoolBuilder::new().num_threads(capped).build()?;
        Ok((Some(pool), Some(capped)))
    } else {
        Ok((None, None))
    }
}
