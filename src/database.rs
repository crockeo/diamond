use rusqlite::Connection;
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Database> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Self {
            conn: Connection::open(path)?,
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        self.conn.execute(
            "
            CREATE TABLE IF NOT EXISTS branches (
                name TEXT PRIMARY KEY,
                parent TEXT
            )
            ",
            (),
        )?;
        Ok(())
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
}
