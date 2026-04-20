//! Core type definitions for the TUI app.

/// Newtype index into the domain registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainId(pub usize);

/// Which view is currently active in the main content area.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    Install,
    Domain(DomainId),
}

/// Three-column focus state.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Focus {
    /// 1열: 도메인 목록
    Sidebar,
    /// 2열: 섹션 메뉴 (up/down로 섹션 이동)
    SectionMenu,
    /// 3열: 선택 섹션 콘텐츠 (up/down로 아이템 이동)
    Content,
}

/// One entry in the sidebar flat list.
#[derive(Clone)]
pub enum SidebarItem {
    GroupHeader(String),
    Install,
    Domain { id: DomainId, label: String, icon: String },
}
