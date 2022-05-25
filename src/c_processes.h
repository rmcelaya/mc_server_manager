#pragma once

#include <stdio.h>
#include <sys/types.h>
#include <unistd.h>

/*

This header file is not used, it only exists to have a reference to import the functions implented in C
into Rust

*/

enum ErrorCode
{
    E_PIPE = -1,
    E_FORK = -2,
    E_WRITE_IO = -3,
    E_READ_EOF = -4,
    E_READ_IO = -5,
};

typedef struct ProcessDescriptor
{
    pid_t proc_pid;
    int proc_stdin;
    int proc_stdout;
    int proc_stderr;
} ProcessDescriptor;

typedef enum KillLevel
{
    K_SIGTERM = 0,
    K_SIGKILL = 1,

} KillLevel;

int c_execute(char *command, char **arguments, char *server_directory, ProcessDescriptor *d, int *e_info, int pipe_input, int pipe_output, int pipe_err);
ssize_t c_write(int fd, void *buff, size_t size, int *error_info);
ssize_t c_read(int fd, void *buff, size_t size, int *error_info);
void c_kill(pid_t pid, KillLevel l);
int c_wait(pid_t pid);
void c_wait_forever(pid_t pid);