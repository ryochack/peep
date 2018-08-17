/// Create a CSI-introduced sequence.
macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

/// Derive a CSI sequence struct.
macro_rules! derive_csi_sequence {
    ($doc:expr, $name:ident, $value:expr) => {
        #[doc = $doc]
        #[derive(Copy, Clone)]
        pub struct $name;

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, csi!($value))
            }
        }
    };
}

pub mod cursor_ext {
    //! Cursor extend movement

    use std::fmt;

    /// Move cursor next line.
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct NextLine(pub u16);

    impl fmt::Display for NextLine {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, csi!("{}E"), self.0)
        }
    }

    /// Move cursor previous line.
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct PreviousLine(pub u16);

    impl fmt::Display for PreviousLine {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, csi!("{}F"), self.0)
        }
    }

    /// Move cursor horizontal absolute.
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct HorizontalAbsolute(pub u16);

    impl fmt::Display for HorizontalAbsolute {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, csi!("{}G"), self.0)
        }
    }
}

pub mod cursor_style {
    //! Cursor style

    use std::fmt;

    derive_csi_sequence!("Set the cursor style blinking block", BlinkingBlock, "1 q");
    derive_csi_sequence!("Set the cursor style steady block", SteadyBlock, "2 q");
    derive_csi_sequence!("Set the cursor style blinking underline", BlinkingUnderline, "3 q");
    derive_csi_sequence!("Set the cursor style steady underline", SteadyUnderline, "4 q");
    derive_csi_sequence!("Set the cursor style blinking bar", BlinkingBar, "5 q");
    derive_csi_sequence!("Set the cursor style steady bar", SteadyBar, "6 q");
}
