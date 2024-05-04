use rusqlite::{Connection, Row, OptionalExtension};
use std::path::Path;

// TODO: WOW is this brittle!!!
// if i add anything earlier into the migration list (why would I?)
// it messes up the revision ordering
const MIGRATIONS: &[&'static str] = &[
    "
    CREATE TABLE IF NOT EXISTS repo_info (
        id INT PRIMARY KEY,
        remote TEXT
    )
    ",
    "
    CREATE TABLE IF NOT EXISTS branches (
        name TEXT PRIMARY KEY,
        parent TEXT
    )
    ",
    "
    ALTER TABLE branches
    ADD submitted BOOL DEFAULT FALSE NOT NULL
    ",
];

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Database> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut db = Self {
            conn: Connection::open(path)?,
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> anyhow::Result<()> {
        self.conn.execute(
            "
            CREATE TABLE IF NOT EXISTS migration (
                id INT PRIMARY KEY,
                current_revision INT
            )
            ",
            (),
        )?;
        for (revision, migration) in MIGRATIONS.iter().enumerate() {
            let transaction = self.conn.transaction()?;
            let current_revision: Option<usize> = transaction.query_row(
                "SELECT current_revision FROM migration WHERE id = 1",
                (),
                |row| row.get(0),
            ).optional()?;
            if let Some(current_revision) = current_revision {
                if current_revision >= revision {
                    continue;
                }
            }

            transaction.execute(migration, ())?;
            transaction.execute(
                "
                INSERT OR REPLACE INTO migration (
                    id,
                    current_revision
                )
                VALUES ( 1, ? )
                ",
                (revision,),
            )?;
            transaction.commit()?;
        }
        Ok(())
    }

    pub fn set_remote(&mut self, remote: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "
            INSERT OR REPLACE INTO repo_info (
                id,
                remote
            ) VALUES (
                1,
                ?
            )
            ",
            (remote,),
        )?;
        Ok(())
    }

    pub fn get_remote(&self) -> anyhow::Result<String> {
        Ok(self
            .conn
            .query_row("SELECT remote FROM repo_info WHERE id = 1", (), |row| {
                row.get(0)
            })?)
    }

    pub fn set_root_branch(&mut self, root_branch: &str) -> anyhow::Result<()> {
        let transaction = self.conn.transaction()?;
        let existing_root_branch: Option<String> = {
            let mut stmt = transaction.prepare("SELECT name FROM branches WHERE parent IS NULL")?;
            let rows = stmt.query_map((), |row| row.get(0))?;
            let mut root_branches: Vec<String> = Vec::new();
            for row in rows {
                let row = row?;
                root_branches.push(row);
            }
            anyhow::ensure!(
                root_branches.len() < 2,
                "Must have 0 or 1 root branches, not {}",
                root_branches.len()
            );
            root_branches.pop()
        };
        if let Some(ref existing_root_branch) = existing_root_branch {
            let num_children: usize = transaction.query_row(
                "SELECT COUNT(*) FROM branches WHERE parent = ?",
                (existing_root_branch,),
                |row| row.get(0),
            )?;
            if num_children > 0 {
                anyhow::bail!("Cannot change root branch when there is an existing root branch with active children.");
            }
            transaction.execute(
                "DELETE FROM BRANCHES WHERE name = ?",
                (existing_root_branch,),
            )?;
        };
        transaction.execute("INSERT INTO branches ( name ) VALUES ( ? )", (root_branch,))?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_root_branch(&self) -> anyhow::Result<String> {
        Ok(self.conn.query_row(
            "SELECT name FROM branches WHERE parent IS NULL",
            (),
            |row| row.get(0),
        )?)
    }

    pub fn get_parent(&self, branch: &str) -> anyhow::Result<Option<String>> {
        Ok(self.conn.query_row(
            "SELECT parent FROM branches WHERE name = ?",
            (branch,),
            |row| row.get(0),
        ).optional()?)
    }

    pub fn create_branch(&mut self, current_branch: &str, new_branch: &str) -> anyhow::Result<()> {
        let transaction = self.conn.transaction()?;
        let current_branch_exists: bool = {
            let count: usize = transaction.query_row(
                "SELECT COUNT(*) FROM branches WHERE name = ?",
                (current_branch,),
                |row| row.get(0),
            )?;
            count > 0
        };
        anyhow::ensure!(
            current_branch_exists,
            "Cannot create branch on top of {current_branch}, which is not tracked."
        );

        transaction.execute(
            "
            INSERT INTO BRANCHES (
                name,
                parent
            ) VALUES (
                ?,
                ?
            )
            ",
            (new_branch, current_branch),
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_branches_in_stack(&mut self, current_branch: &str) -> anyhow::Result<Vec<String>> {
        let transaction = self.conn.transaction()?;
        let parent: String = transaction.query_row(
            "SELECT parent FROM branches WHERE name = ?",
            (current_branch,),
            |row| row.get(0),
        )?;
        let mut stmt = transaction.prepare(
            "
            WITH RECURSIVE
              stack_branches(name, parent) AS (
                VALUES(?, ?)
                UNION
                SELECT branches.name, branches.parent
                FROM branches, stack_branches
                WHERE branches.parent = stack_branches.name
                   OR branches.name = stack_branches.parent
              )
            SELECT name
            FROM stack_branches
            WHERE parent IS NOT NULL
            ",
        )?;
        let mut branches: Vec<String> = Vec::new();
        for row in stmt.query_map((current_branch, parent), |row| row.get(0))? {
            let row = row?;
            branches.push(row);
        }
        Ok(branches)
    }
}
