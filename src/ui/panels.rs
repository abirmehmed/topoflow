//! UI panels for the application

pub struct Toolbar {
    pub current_tool: Tool,
}

pub enum Tool {
    Select,
    Move,
    Rotate,
    Scale,
    Draw,
    Relax,
    Slide,
}

pub struct PropertiesPanel {
    pub target_faces: usize,
    pub symmetry_enabled: bool,
    pub symmetry_axis: SymmetryAxis,
}

pub enum SymmetryAxis {
    X,
    Y,
    Z,
}

pub struct OutlinerPanel {
    // TODO: Object tree
}
