#ifndef STOW_H
#define STOW_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define STOW_OK 0
#define STOW_ERR_INVALID_ARGUMENT 1
#define STOW_ERR_IO 2
#define STOW_ERR_INTERNAL 3
#define STOW_ERR_INVALID_MAGIC 16
#define STOW_ERR_UNSUPPORTED_VERSION 17
#define STOW_ERR_TRUNCATED 18
#define STOW_ERR_CORRUPT 19
#define STOW_ERR_CHECKSUM 20
#define STOW_ERR_UNKNOWN_CODEC 21
#define STOW_ERR_INVALID_NAME 22
#define STOW_ERR_DUPLICATE_NAME 23
#define STOW_ERR_TOO_LARGE 24
#define STOW_ERR_INDEX_OUT_OF_RANGE 25

typedef struct StowWriter StowWriter;
typedef struct StowReader StowReader;

const char *stow_version(void);
const char *stow_error_message(int error);
void stow_buffer_free(uint8_t *data, size_t len);

StowWriter *stow_writer_new(void);
void stow_writer_free(StowWriter *writer);
int stow_writer_add_mem(StowWriter *writer, const char *name, const uint8_t *data, size_t len);
int stow_writer_add_file(StowWriter *writer, const char *path, const char *name);
int stow_writer_finish(StowWriter *writer, uint8_t **out, size_t *out_len);
int stow_writer_save(StowWriter *writer, const char *out_path);

int stow_reader_open_mem(const uint8_t *data, size_t len, StowReader **out);
int stow_reader_open_file(const char *path, StowReader **out);
void stow_reader_free(StowReader *reader);
size_t stow_reader_count(const StowReader *reader);
const char *stow_reader_name(const StowReader *reader, size_t index);
uint64_t stow_reader_size(const StowReader *reader, size_t index);
int stow_reader_read(const StowReader *reader, size_t index, uint8_t **out, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif
