## One Branch Per Assignment, refer to the HomeworkX.md file for instructions

The assignments in this class are all represented as branches in this repository. 
To avoid merge conflicts in the README file as we update the assignments, each assignment writeup is in its own separate HomeworkX.md file. 

Switch to the branch you need, then refer to the HomeworkX.md file for assignment instructions. 

## General Instructions

To build the server, use ``cargo build --bin server`` or ``cargo build --bin server --release`` for the optimized version. Similarly, to build the benchmark, use ``--bin benchmark``. Alternatively, use ``run`` instead of ``build`` to directly run the program. If you want to pass arguments to the program after a ``cargo run``, put an extra ``--`` before the arguments thus: ``cargo run --bin server -- -m``.

When using profiling tools, it is often better and less confusing to run the program without using ``cargo run``. Both ``run`` and ``build`` produce a binary in ``target/debug/`` or ``target/release`` depending on if you passed the ``--release`` flag to ``cargo``. Thus, you can run the produced server executable directly using ``target/release/server``. The extra ``--`` is not used when running the executable directly. 

Each program has several options, which you may inspect using the ``--help``. 

## Special profiling support in ``benchmark`` and ``server``

### --exitcode

There are some special facilities provided to make profiling these programs easier. First, the ``--exitcode`` argument. Use it to pass a code word to the server. Use it to pass the same code word to the benchmark program. When the benchmark program finishes its work, it passes the exit code to the server, making the server exit. If you are profiling the server, having it exit automatically at the end of a run is very convenient.

### --sleeperfile

The ``--sleeperfile`` argument to ``benchmark`` is meant to work together with ``filesleep.sh``. Here, ``filesleep.sh`` creates a temporary file using the name passed as its first argument. It then waits indefinitely, until the file disappears. Pass the name of the file to the benchmark program using the ``--sleeperfile`` argument, and the benchmark program will delete the file when it starts running the performance test. 

You can use this to postpone collecting profiling data until after the performance experiment has begun, which avoids contaminating the profiling data with work done during initialization. For example, use

``./filesleep.sh mysleeperfile ; ./mymeasurement``

to run ``./mymeasurement`` once the benchmark has started running the performance test. 

### server_pid.txt

The server automatically stores its process identifier (PID) in the file ``server_pid.txt``. You may use this to attach profiling tools or ``gdb`` to an already running process. For example

``./filesleep.sh mysleeperfile ; gdb -p $(cat server_pid.txt)``

will attach ``gdb`` to the server process after the performance measurement has begun. 
