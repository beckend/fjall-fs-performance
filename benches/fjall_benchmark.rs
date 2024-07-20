#[path = "./common.rs"]
mod common;
#[path = "./db_test.rs"]
mod db_test;

use bytes::Bytes;
use color_eyre::{Report, Result};
use common::{get_db_paths, get_path_prefix, BENCH_ITEMS_AMOUNT};
use criterion::{criterion_group, criterion_main, Criterion};
use db_test::{handle_on_drop_remove_dir, DBTest};
use fjall::{TransactionalKeyspace, TxKeyspace};
use std::{
  borrow::Cow,
  future::Future,
  path::Path,
  time::{Duration, Instant},
};
use tokio::{fs, runtime::Runtime};

pub fn get_db_fs_inner_fjall(
  path: impl AsRef<Path>,
  remove_on_drop: bool,
) -> Result<DBTest<TxKeyspace>, Report> {
  let path = path.as_ref().to_owned();

  Ok(DBTest::new(
    // Seems like create and open is one and the same in fjall.
    fjall::Config::new(&path).open_transactional()?,
    if remove_on_drop {
      Some(Box::new(move || {
        handle_on_drop_remove_dir(path);
      }))
    } else {
      None
    },
  ))
}

static PARTITION: &str = "partition_main";

#[allow(clippy::ptr_arg)]
fn mapper(
  path: impl AsRef<Path> + ToOwned,
) -> impl Future<Output = Result<DBTest<TransactionalKeyspace>, Report>> {
  let path = Cow::from(path.as_ref().to_owned());
  let fn_create_open = get_db_fs_inner_fjall;

  async move {
    tokio::task::spawn_blocking(move || {
      let start = Instant::now();
      let db = fn_create_open(path.clone(), false)?;
      let partition = db.open_partition(PARTITION, Default::default())?;

      {
        let mut write_txn = db.write_tx();

        for i in 0..BENCH_ITEMS_AMOUNT {
          let v = Bytes::from(format!("test{i}"));
          write_txn.insert(&partition, v.as_ref(), v.as_ref());
        }
        write_txn.commit()?;
      }

      db.persist(fjall::PersistMode::SyncAll)?;
      eprintln!("fjall {path:?} done: {:?}", start.elapsed());

      Ok::<_, Report>(db)
    })
    .await?
  }
}

async fn work() -> Result<()> {
  let path_temp = get_path_prefix("cargo_bench_fjall");
  let paths = get_db_paths(&path_temp, 1);
  fs::create_dir_all(&path_temp).await?;
  let start = Instant::now();

  for _ in 0..2 {
    let tasks = paths.iter().map(mapper).collect::<Vec<_>>();

    for x in tasks {
      x.await?;
    }
  }
  eprintln!("fjall all done: {:?}", start.elapsed());

  fs::remove_dir_all(&path_temp).await?;
  Ok(())
}

fn bench(c: &mut Criterion) {
  let rt = Runtime::new().expect("Create runtime.");

  c.bench_function("fjall", |b| {
    b.to_async(&rt).iter(work);
  });
}

criterion_group! {
    name = benches;
    config = Criterion::default().warm_up_time(Duration::from_nanos(1)).sample_size(10);
    targets = bench
}
criterion_main!(benches);
