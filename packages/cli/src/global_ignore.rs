use std::{collections::HashMap, path::PathBuf};

use ignore::gitignore::Gitignore;

pub struct GlobalIgnore {
    pub krates: HashMap<PathBuf, KrateWithIgnore>,
}

pub struct KrateWithIgnore {
    pub(crate) manifest_path: PathBuf,
    pub(crate) ignore: Gitignore,
}
