mod internal;
mod nodes;

use godot::prelude::*;

pub use nodes::*;

pub struct HttpServerExtension;

#[gdextension]
unsafe impl ExtensionLibrary for HttpServerExtension {}
