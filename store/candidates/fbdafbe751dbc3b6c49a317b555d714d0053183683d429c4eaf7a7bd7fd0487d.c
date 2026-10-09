void w2m_set_last_error(u32 code) {
    w2m_teb()->last_error_value = code;
}

u32 w2m_get_last_error(void) {
    return w2m_teb()->last_error_value;
}
