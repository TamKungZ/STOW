#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "stow.h"

static int check(int rc, const char *what) {
    if (rc != STOW_OK) {
        fprintf(stderr, "%s failed: %s\n", what, stow_error_message(rc));
        return 1;
    }
    return 0;
}

int main(void) {
    const char *text = "A84R5A84R5A84R5A84R5A84R5A84R5A84R5A84R5A84R5";
    uint8_t zeros[2048];
    memset(zeros, 0, sizeof zeros);

    StowWriter *w = stow_writer_new();
    if (!w) return 1;
    if (check(stow_writer_add_mem(w, "text/a.txt", (const uint8_t *)text, strlen(text)), "add a")) return 1;
    if (check(stow_writer_add_mem(w, "zeros.bin", zeros, sizeof zeros), "add zeros")) return 1;
    if (check(stow_writer_add_mem(w, "empty", NULL, 0), "add empty")) return 1;
    if (stow_writer_add_mem(w, "../evil", zeros, 1) != STOW_ERR_INVALID_NAME) return 1;
    if (stow_writer_add_mem(w, "zeros.bin", zeros, 1) != STOW_ERR_DUPLICATE_NAME) return 1;

    uint8_t *buf = NULL;
    size_t buf_len = 0;
    if (check(stow_writer_finish(w, &buf, &buf_len), "finish")) return 1;
    stow_writer_free(w);

    StowReader *r = NULL;
    if (check(stow_reader_open_mem(buf, buf_len, &r), "open")) return 1;
    if (stow_reader_count(r) != 3) return 1;
    if (strcmp(stow_reader_name(r, 0), "text/a.txt") != 0) return 1;
    if (stow_reader_size(r, 1) != sizeof zeros) return 1;
    if (stow_reader_name(r, 3) != NULL) return 1;

    uint8_t *out = NULL;
    size_t out_len = 0;
    if (check(stow_reader_read(r, 0, &out, &out_len), "read 0")) return 1;
    if (out_len != strlen(text) || memcmp(out, text, out_len) != 0) return 1;
    stow_buffer_free(out, out_len);

    if (check(stow_reader_read(r, 1, &out, &out_len), "read 1")) return 1;
    if (out_len != sizeof zeros || memcmp(out, zeros, out_len) != 0) return 1;
    stow_buffer_free(out, out_len);

    if (check(stow_reader_read(r, 2, &out, &out_len), "read 2")) return 1;
    if (out_len != 0) return 1;
    stow_buffer_free(out, out_len);

    if (stow_reader_read(r, 9, &out, &out_len) != STOW_ERR_INDEX_OUT_OF_RANGE) return 1;
    stow_reader_free(r);

    StowReader *bad = (StowReader *)1;
    if (stow_reader_open_mem((const uint8_t *)"not an archive", 14, &bad) != STOW_ERR_INVALID_MAGIC) return 1;
    if (bad != NULL) return 1;

    printf("stow %s: archive %zu bytes, c abi roundtrip ok\n", stow_version(), buf_len);
    stow_buffer_free(buf, buf_len);
    return 0;
}
