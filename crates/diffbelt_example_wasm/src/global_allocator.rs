use dlmalloc::GlobalDlmalloc;

#[global_allocator]
static GLOBAL: GlobalDlmalloc = GlobalDlmalloc;
