use bitflags::bitflags;

pub const BORDERS: BorderSet = BorderSet {
    top_left_qorner: '┌',
    top_right_qorner: '┐',
    bot_left_qorner: '└',
    bot_right_qorner: '┘',
    vertical_left: '│',
    vertical_right: '│',
    horizontal_top: '─',
    horizontal_bot: '─',
};

pub const DOUBLE_BORDERS: BorderSet = BorderSet {
    top_left_qorner: '╔',
    top_right_qorner: '╗',
    bot_left_qorner: '╚',
    bot_right_qorner: '╝',
    vertical_left: '║',
    vertical_right: '║',
    horizontal_top: '═',
    horizontal_bot: '═',
};

pub const THICK_BORDERS: BorderSet = BorderSet {
    top_left_qorner: '┏',
    top_right_qorner: '┓',
    bot_left_qorner: '┗',
    bot_right_qorner: '┛',
    vertical_left: '┃',
    vertical_right: '┃',
    horizontal_top: '━',
    horizontal_bot: '━',
};

pub const HAVED_WIDE_BORDERS: BorderSet = BorderSet {
    top_left_qorner: '▛',
    top_right_qorner: '▜',
    bot_left_qorner: '▙',
    bot_right_qorner: '▟',
    vertical_left: '▌',
    vertical_right: '▐',
    horizontal_top: '▀',
    horizontal_bot: '▄',
};

pub const HAVED_THIN_BORDERS: BorderSet = BorderSet {
    top_left_qorner: '▗',
    top_right_qorner: '▖',
    bot_left_qorner: '▝',
    bot_right_qorner: '▘',
    vertical_left: '▐',
    vertical_right: '▌',
    horizontal_top: '▄',
    horizontal_bot: '▀',
};

pub const FULL_BORDERS: BorderSet = BorderSet {
    top_left_qorner: '█',
    top_right_qorner: '█',
    bot_left_qorner: '█',
    bot_right_qorner: '█',
    vertical_left: '█',
    vertical_right: '█',
    horizontal_top: '█',
    horizontal_bot: '█',
};

pub const HAVLED_BALANCED_BORDERS: BorderSet = BorderSet {
    top_left_qorner: '▄',
    top_right_qorner: '▄',
    bot_left_qorner: '▀',
    bot_right_qorner: '▀',
    vertical_left: '█',
    vertical_right: '█',
    horizontal_top: '▄',
    horizontal_bot: '▀',
};

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
    #[derive(Default, Clone, Copy, Eq, PartialEq, Hash, Debug)]
    pub struct Borders: u8 {
        /// Show no border (default)
        const NONE   = 0b0000;
        /// Show the top border
        const TOP    = 0b0001;
        /// Show the right border
        const RIGHT  = 0b0010;
        /// Show the bottom border
        const BOTTOM = 0b0100;
        /// Show the left border
        const LEFT   = 0b1000;
        /// Show all borders
        const ALL = Self::TOP.bits() | Self::RIGHT.bits() | Self::BOTTOM.bits() | Self::LEFT.bits();
    }
}

pub struct BorderSet {
    pub top_left_qorner: char,
    pub top_right_qorner: char,
    pub bot_left_qorner: char,
    pub bot_right_qorner: char,
    pub vertical_left: char,
    pub vertical_right: char,
    pub horizontal_top: char,
    pub horizontal_bot: char,
}

impl BorderSet {
    pub const fn double() -> Self {
        DOUBLE_BORDERS
    }
}
