#[derive(Clone, Debug, PartialEq)]
pub enum PeepEvent {
    MoveDown(u16),
    MoveUp(u16),
    MoveLeft(u16),
    MoveRight(u16),
    MoveDownHalfPages(u16),
    MoveUpHalfPages(u16),
    MoveLeftHalfPages(u16),
    MoveRightHalfPages(u16),
    MoveDownPages(u16),
    MoveUpPages(u16),
    MoveToHeadOfLine,
    MoveToEndOfLine,
    MoveToTopOfLines,
    MoveToBottomOfLines,
    MoveToLineNumber(u16),

    ToggleLineNumberPrinting,
    ToggleLineWraps,
    IncrementLines(u16),
    DecrementLines(u16),
    SetNumOfLines(u16),

    SearchIncremental(String),
    SearchTrigger,
    SearchNext,
    SearchPrev,

    Message(Option<String>),

    Cancel,
    Quit,

    FollowMode,
    FileUpdated,
    SigInt,
}
