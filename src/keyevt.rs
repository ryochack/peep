/// Key Event Handler

#[derive(Clone, Debug, PartialEq)]
pub enum KeyOp {
    MoveDown(u32),
    MoveUp(u32),
    MoveLeft(u32),
    MoveRight(u32),
    MoveDownHalfPages(u32),
    MoveUpHalfPages(u32),
    MoveDownPages(u32),
    MoveUpPages(u32),
    MoveToHeadOfLine,
    MoveToEndOfLine,
    MoveToTopOfLines,
    MoveToBottomOfLines,
    MoveToLineNumber(u32),

    ShowLineNumber(bool),
    ShowNonPrinting(bool),
    IncrementLines(u32),
    DecrementLines(u32),
    SetNumOfLines(u32),

    SearchNext,
    SearchPrev,
    SearchIncremental(String),

    Message(String),

    Cancel,
    Quit,
}
