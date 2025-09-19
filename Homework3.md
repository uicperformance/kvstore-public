In this third assignment, we are starting with a solution for hw2, with some minor additional changes, including 
setting the server to single-threaded by default. We'll return to studying multi-threaded server performance later. 

Compared to the first two assignments, this assignment focuses primarily on understanding the performance
behavior of the program, and the sources of performance degradation. 

## 1. Observation - more client threads makes the program run slower

Unfortunately, while the hw3 code is much better than the hw2 code, addressing synchronization issues and efficient batch processing, there remains a significant performance problem. 

To see the problem, start the server using only the ``--addr`` and ``--exit-code`` parameters.

Benchmark raw:
        --- Benchmark Results ---
        Total Operations: 10000
        Total Duration:   42.5727 s
        Avg Operations/s: 234.89
        Throughput:       0.0002 M req/s

        Executed in   38.59 secs    fish           external
           usr time    1.04 secs  591.00 micros    1.04 secs  35%
           sys time    1.89 secs  358.00 micros    1.89 secs  65%

Benchmark 1 thread:
        --- Benchmark Results ---
        Total Operations: 1000
        Total Duration:   1.7980 s
        Avg Operations/s: 556.16
        Throughput:       0.0006 M req/s

        Executed in    2.33 secs      fish           external
           usr time   19.99 millis  474.00 micros   19.52 millis 18%
           sys time   91.37 millis  284.00 micros   91.09 millis 82%

Benchmark 2 thread:
        --- Benchmark Results ---
        Total Operations: 2000
        Total Duration:   4.0348 s
        Avg Operations/s: 495.68
        Throughput:       0.0005 M req/s

        Executed in    4.47 secs      fish           external
           usr time   68.83 millis  435.00 micros   68.39 millis 23%
           sys time  229.88 millis  262.00 micros  229.61 millis 77%


Benchmark 3 thread:
        --- Benchmark Results ---
        Total Operations: 3000
        Total Duration:   6.8261 s
        Avg Operations/s: 439.49
        Throughput:       0.0004 M req/s

        Executed in    7.28 secs      fish           external
           usr time  198.20 millis  789.00 micros  197.41 millis 35%
           sys time  369.08 millis    0.00 micros  369.08 millis 65%



Then, run the benchmark program using the above parameters as well as ``--threads 1``. Then, run the same 
experiment, but now with ``--threads 2``, and ``--threads 4``. The more client threads we add, the lower the measured performance. 

    % time     seconds  usecs/call     calls    errors syscall
    ------ ----------- ----------- --------- --------- ------------------
     74.01   26.790909        5343      5014           close
     20.54    7.434208        1494      4974           openat
      3.24    1.171650       55792        21           accept4
      2.17    0.785436         157      4973           write
      0.03    0.011302          62       180           recvfrom
      0.01    0.003610          35       101           sendto
      0.00    0.000706         705         1           execve
      0.00    0.000457          14        32           brk
      0.00    0.000448          29        15           munmap
      0.00    0.000306          11        26           mmap
      0.00    0.000198           9        22           setsockopt
      0.00    0.000180           8        21           fcntl
      0.00    0.000086          14         6           mprotect
      0.00    0.000071          14         5           read
      0.00    0.000048           9         5           rt_sigaction
      0.00    0.000044          10         4           newfstatat
      0.00    0.000040           9         4           pread64
      0.00    0.000027           8         3           sigaltstack
      0.00    0.000023          11         2         1 arch_prctl
      0.00    0.000023          22         1           socket
      0.00    0.000020          10         2           prlimit64
      0.00    0.000016          15         1         1 access
      0.00    0.000014          13         1           bind
      0.00    0.000013          12         1           poll
      0.00    0.000011          11         1           getrandom
      0.00    0.000010          10         1           listen
      0.00    0.000010          10         1           sched_getaffinity
      0.00    0.000010           9         1           getpid
      0.00    0.000010           9         1           rseq
      0.00    0.000009           9         1           set_robust_list
      0.00    0.000009           9         1           set_tid_address
    ------ ----------- ----------- --------- --------- ------------------
    100.00   36.199902        2347     15422         2 total

_Without_ doing a line by line comparison of the source code (including _not_ using git diff and such), use profiling tools to get further clues as to why the server is now so absurdly slow. 

- [x] Does ``time`` give you any useful clues? Is it spending more time waiting, working in user space, or in the kernel? How does this proportion change with the number of client threads?
- [ ] Based on what ``time`` told you, you'll want to again either use ``perf record`` or ``strace`` to see what we were waiting for. 
- [ ] Try running the server with ``--dbfile /tmp/$USER.db`` where ``$USER`` is your username. Think of ``/tmp/`` as very fast storage for now. Storing the database there makes server substantially faster, but if you increase to ``--ops 10000``, you'll find that the thread slowdown trend persists even with ``/tmp/`` storage. 
- [ ] Note: A lesser version of the problem even occurs with ``-m`` (mem-only database), but let's focus on the storage-based version for now. 

## 2. Track down the culprit

With the ``/tmp/`` storage option, you'll find that about 50% of the CPU time is spent in userspace, and 50% in the kernel. Use both ``strace`` and ``perf record/report`` to explore this further. 

We don't know how fast this program should run. All we know is that it's running slower when we add more threads to the benchmark client. To track it down, try to look for what changes as we vary the number of threads. Of course we do more work, so don't let the changing totals confuse you. However, some of that work also appears to be taking longer with more threads. 

Handy tool 1: ``perf record -o`` and ``perf report -i`` let you specify a data file, rather than use the default ``perf.data``. This way, you can save your results for later. Note, however, that if you recompile your binary, the data is unlikely to remain useful. ``perf diff`` compares two recordings, telling you what changed, and how much. 

Handy tool 2: ``strace -e`` lets you specify specific system calls you are interested in, rather than get a huge list every time. 
``write`` is a big item in the ``strace -cf`` list. How does that change with the number of threads? Try running ``strace -e write``, to see
each system call as it flies by. With ``-e`` it can yield some useful information. You can also try ``strace -ek`` to get a backtrace for each system call. 

Unfortunately, due to some unknown compatibility issue, doesn't currently report line numbers on the course server. A more robust approach is to catch it in ``gdb`` instead. **Helpful refresher:** Use ``gdb --args`` and then the full server command line (not the ``cargo`` one, just the server binary and its arguments) to start. Then use ``break`` to put a breakpoint at an interesting function, like ``break close`` for example. ``run`` to start running the program, and ``bt`` for a backtrace. ``cont`` to continue to the next breakpoint. 

## 3. Turn-in

As your turn-in for this assignment, complete the assignment "Hw3 turn-in" on gradescope. Recall that the value of this turn-in is very small compared to the value of doing the work so that you are prepared for the quiz or computer exam. You likely have more to gain by challenging yourself to working out these answers without help, and even without ``git diff``.

## 4. Evaluation

The evaluation for this assignment will be an in-class paper quiz, or computer exam. The quiz/computer exam will cover material up to hw3, and material leading up to it, though the focus will be on recent material. 
