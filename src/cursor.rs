//! Cursor extend movement

/// Move cursor next line.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct NextLine(pub u16);

impl fmt::Dispaly for NextLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}E"), self.0)
    }
}

/// Move cursor previous line.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PreviousLine(pub u16);

impl fmt::Dispaly for PreviousLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}F"), self.0)
    }
}

/// Move cursor horizontal absolute.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct HorizontalAbsolute(pub u16);

impl fmt::Dispaly for HorizontalAbsolute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}G"), self.0)
    }
}

derive_csi_sequence!("Set cursor style blinking block", BlinkBlock, "0 q");
derive_csi_sequence!("Set cursor style steady block", BlinkBlock, "0 q");
