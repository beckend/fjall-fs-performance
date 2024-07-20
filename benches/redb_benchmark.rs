#[path = "./common.rs"]
mod common;
#[path = "./db_test.rs"]
mod db_test;

use bytes::Bytes;
use color_eyre::{Report, Result};
use common::{get_db_paths, get_path_prefix, BENCH_ITEMS_AMOUNT};
use criterion::{criterion_group, criterion_main, Criterion};
use db_test::{handle_on_drop_remove_dir, DBTest};
use redb::{Database, TableDefinition};
use std::{
  borrow::Cow,
  future::Future,
  path::Path,
  time::{Duration, Instant},
};
use tokio::{fs, runtime::Runtime};

pub fn get_db_fs_inner_redb(
  path: impl AsRef<Path>,
  create: bool,
  remove_on_drop: bool,
) -> Result<DBTest<Database>, Report> {
  let path = path.as_ref().to_owned();

  Ok(DBTest::new(
    if create {
      redb::Builder::new().create(&path)?
    } else {
      redb::Builder::new().open(&path)?
    },
    if remove_on_drop {
      Some(Box::new(move || {
        handle_on_drop_remove_dir(path);
      }))
    } else {
      None
    },
  ))
}

const TABLE: TableDefinition<&'static [u8], &'static [u8]> = TableDefinition::new("my_data");

#[allow(clippy::ptr_arg)]
fn mapper(
  path: impl AsRef<Path> + ToOwned,
) -> impl Future<Output = Result<DBTest<Database>, Report>> {
  let path = Cow::from(path.as_ref().to_owned());
  let fn_create_open = get_db_fs_inner_redb;

  async move {
    tokio::task::spawn_blocking(move || {
      let start = Instant::now();
      let mut db = fn_create_open(path.clone(), true, false)?;

      {
        let mut write_txn = db.begin_write()?;
        write_txn.set_durability(redb::Durability::Immediate);

        {
          let mut table = write_txn.open_table(TABLE)?;
          for i in 0..BENCH_ITEMS_AMOUNT {
            let v = Bytes::from(format!("test{i}"));
            table.insert(v.as_ref(), v.as_ref())?;
          }
        }
        write_txn.commit()?;
      }

      db.compact()?;
      eprintln!("redb {path:?} done: {:?}", start.elapsed());

      Ok::<_, Report>(db)
    })
    .await?
  }
}

async fn async_work() -> Result<()> {
  tokio::time::sleep(Duration::from_millis(100)).await;
  let path_temp = get_path_prefix("cargo_bench_redb");
  let paths = get_db_paths(&path_temp, 1);
  fs::create_dir_all(&path_temp).await?;
  let start = Instant::now();

  for _ in 0..2 {
    let tasks = paths.iter().map(mapper).collect::<Vec<_>>();

    for x in tasks {
      x.await?;
    }
  }
  eprintln!("redb all done: {:?}", start.elapsed());

  fs::remove_dir_all(&path_temp).await?;
  Ok(())
}

fn bench(c: &mut Criterion) {
  let rt = Runtime::new().expect("Create runtime.");

  c.bench_function("redb", |b| {
    b.to_async(&rt).iter(|| async { async_work().await });
  });
}

criterion_group! {
    name = benches;
    config = Criterion::default().warm_up_time(Duration::from_nanos(1)).sample_size(10);
    targets = bench
}
criterion_main!(benches);
