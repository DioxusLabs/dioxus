rsx! {
    Pagination {
        PaginationContent {
            PaginationPrevious { onclick: move |_| on_prev(()) }
            PaginationNext { onclick: move |_| on_next(()) }
        }
    }
}
