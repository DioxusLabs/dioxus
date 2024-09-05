//! Methods for working with the filesystem that are faster than the std fs methods
//! Uses stuff like rayon, caching, and other optimizations
//!
//! Allows configuration in case you want to do some work while copying and allows you to track progress
