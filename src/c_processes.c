#include <stdlib.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <string.h>
#include <stdio.h>
#include <errno.h>

#include "c_processes.h"

int execute(char *command, char **arguments, char *server_directory, ProcessDescriptor *d, int *e_info,
            int pipe_input, int pipe_output, int pipe_err)
{
    int fd_in[2];
    int fd_out[2];
    int fd_err[2];

    if (pipe_input == 1)
    {
        if (pipe(fd_in) < 0)
        {
            *e_info = errno;
            return E_PIPE;
        }
    }
    if (pipe_output == 1)
    {
        if (pipe(fd_out) < 0)
        {
            *e_info = errno;
            return E_PIPE;
        }
    }
    if (pipe_err == 1)
    {
        if (pipe(fd_err) < 0)
        {
            *e_info = errno;
            return E_PIPE;
        }
    }

#ifdef DEBUG
    printf("Command: %s\n", command);

    int i = 0;
    while (arguments[i])
    {
        printf("Arg: %s\n", arguments[i]);
        i++;
    }
#endif

    pid_t pid;
    if ((pid = fork()) < 0)
    {
        *e_info = errno;
        return E_FORK;
    }

    if (pid == 0)
    {
        if (pipe_input == 1)
        {
            close(fd_in[1]);
            dup2(fd_in[0], STDIN_FILENO);
            close(fd_in[0]);
        }

        if (pipe_output == 1)
        {
            close(fd_out[0]);
            dup2(fd_out[1], STDOUT_FILENO);
            close(fd_out[1]);
        }

        if (pipe_err == 1)
        {
            close(fd_err[0]);
            dup2(fd_err[1], STDERR_FILENO);
            close(fd_err[1]);
        }

        chdir(server_directory);
        if (execvp(command, arguments) < 0)
        {
            perror("Here is the child process talking. Exec called failed so the process path is probably wrong. Not gonna deal with communicating this to my parent tbh.");
            exit(33);
        }
    }

    if (pipe_input == 1)
    {
        close(fd_in[0]);
        d->proc_stdin = fd_in[1];
    }
    if (pipe_output == 1)
    {
        close(fd_out[1]);
        d->proc_stdout = fd_out[0];
    }
    if (pipe_err == 1)
    {
        close(fd_err[1]);
        d->proc_stderr = fd_err[0];
    }
    d->proc_pid = pid;

    return 0;
}

ssize_t c_write(int fd, void *buff, size_t size, int *error_info)
{

    ssize_t r = write(fd, buff, size);
    if (r == -1)
    {
        *error_info = errno;
        return E_WRITE_IO;
    }
    return r;
}

ssize_t c_read(int fd, void *buff, size_t size, int *error_info)
{

    ssize_t r;

    r = read(fd, buff, size);
    if (r == 0)
    {
        return E_READ_EOF;
    }
    else if (r == -1)
    {
        *error_info = errno;
        return E_READ_IO;
    }

    return r;
}

void c_kill(pid_t pid, KillLevel l)
{
    //We don't look for errors in kill

    switch (l)
    {
    case K_SIGTERM:
        kill(pid, SIGTERM);
        break;
    case K_SIGKILL:
        kill(pid, SIGKILL);
        break;
    default:
        break;
    }
}

//Returns 0 if the call was success, don't mix it with
//the waitpid with WHNHANG call that will return 0 if the
//process exists but is not done yet
int c_wait(pid_t pid)
{

    int e = waitpid(pid, NULL, WNOHANG);
    if (e < 0)
    {
        if (errno == ECHILD)
        {
            return -1;
        }
        else
        {
            printf("Bug found when calling wait. EINVAL found or the call was interrupted by a signal (which should not be happening btw).\n");
            exit(-1);
        }
    }
    return e;
}

void c_wait_forever(pid_t pid)
{
    int e = waitpid(pid, NULL, 0);
    if (e < 0)
    {
        if (errno == ECHILD)
        {
            return;
        }
        else
        {
            printf("Bug found when calling wait. EINVAL found or the call was interrupted by a signal (which should not be happening btw).\n");
            exit(-1);
        }
    }
}