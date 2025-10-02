
In this this first assignment, you are given a basic key-value store server, as well a basic client, and a benchmarking client.

The server is very, very slow. With the default benchmark settings and server, we are getting approx. $25$ operations per second when running on the class server. Over the course of the semester, we are going to make it incredibly fast, both by supporting a wider variety of settings, and by improving the efficiency of the implementation of the server and the underlying data structure. However, for starters, your job is to learn some basic tools and concepts.

The programs are written in Rust - a modern systems programming language. You'll need to pick up some basic rust to understand what the program is doing. Consult your favorite AI, [this excellent book](https://doc.rust-lang.org/book/), or learn by doing with [Rustlings](https://rustlings.rust-lang.org/). 

## 1. Getting started 

You may use the class server, or your own Linux machine. For most assignments, running native on OS X should also be fine. Doing the assignments on Windows (including WSL) is however **strongly discouraged**, as your experience will likely not mirror everyone else's, *particularly in regards to performance*. 

``git clone`` this repository to a folder on your machine of choice. To run, use the following commands in the project folder:

- `cargo run --bin server -- --addr "127.0.0.1:4000"`
    - Here, if port $4000$ is already in use, you may pick a different one: use a number between $1024$ and $32768$.
- Now, in a separate terminal window: `cargo run --bin benchmark -- --threads 1 --connections 1 --batch-size 1 --addr "127.0.0.1:4000"`
>>>>>>> 8e117cf1bf8f6576bb307844e94abe1a9ce9ab28
The ``threads``, ``connections`` and ``batch-size`` parameters control various forms of concurrency within the benchmark client, but we are only interested in ``--batch-size`` in this assignment. There are more controllable parameters for server and benchmark client that you can learn about using the ``--help`` argument, and reading the source code. 

_A note about Rust:_ Rust is similar to C in many ways: it's essentially true that anything you can do in C, you can do in Rust, including achieving the same performance. However, being a modern language, it has many convenience features that C lacks, as well as a robust build system and standard library. 

You won't need to know much about Rust for this assignment, but one key language features is worth pointing out: when a value on the stack goes out of scope in Rust, its memory is automatically freed (just like in C), but before that, if there exists a [`drop()` function implementation](https://doc.rust-lang.org/std/ops/trait.Drop.html#tymethod.drop) for the type, `drop()` is called on the value before the memory is freed. **This applies recursively**, in the case of struct values.

## 2. Use some tools to study the behavior of the programs

Since you are likely new to Rust, simply reading the programs and manually spotting the performance problems is not likely to be fully successful, even for these tiny programs. This in some ways mirrors the experience of trying to understand the performance of a larger program by simply reading the code. You'll need to gear up, learn to use some of the tools of the trade. 

Use ``time`` to see if the benchmark program and server processes are busy working, or mostly waiting. If they're working, is it mostly in user space code, or are they asking the kernel to do a lot of busy work for them? 

If the program is mostly waiting, then it's spending time in system calls. 

Use ``strace`` to learn what system calls the applications are making. The you'll find the ``-c``, ``-w``, ``-k`` and ``-f`` arguments particularly relevant to this assignment. Read up on what these arguments do. If it helps with motivation: knowing what these options do _will_ be on the quiz. 

_Note_: the Futex system call is used to implement thread joins and other synchcronization. It is **not** a significant factor in this program's performance. 

If the program is mostly busy in user space, then you can use ``perf record`` and ``perf report`` to get a statistical estimate of what it's busy doing.

## 3. Find some performance bugs

In this assignment, the performance issues all exist within the file ``server.rs``. While it will be instructive to read ``benchmark.rs`` and ``tree.rs``, the performance bugs we are concerned about in this assignment are not in those files. 

Depending on how you count them, there are a handful of principal performance bugs that we are targeting in this assignment. As you will learn throughout the semester, it's often hard to even know how fast a program *should run*. "Is this program fast?" can be a "How long is a piece of string?" sort of question. Sometimes we may have an intuition, but other times, we can get a clue that something is off from other performance inconsistencies. Here are some clues for this program:

* Prepopulating the database with more entries makes it run much slower
* On the class server, configuring a `/tmp/` database file path makes a huge difference to performance
* Using a batch size larger than $1$ doesn't change anything

When you identify a bug, go ahead and fix it (should be only very small changes, like changing part of a line). 

## 4. Evaluation and turn-in

As a reminder, evaluation for this assignment will be by quiz. There's no turn-in. Instead, think of this as an exercise that will help you perform on the in-person quiz, in lecture after the assignment due date. The quiz will contain questions regarding the tools you are meant to use, the programs we are analyzing, the performance bugs, and the process by which you identified the bugs. 

Thus, you are welcome to work in teams, or work with an AI, to help you learn the tools, read the code, and do the work. However, if you lean too hard on your team-mates, AI or other crutches, you may find yourself not learning much, and bombing the quiz. 

## 4. Evaluation
The main purpose of this assignment is to help you learn what you'll need to know to pass the in-person quizzes, which will be held in-lecture after the assignment due dates.
The quiz will contain questions regarding the tools you are meant to use, the programs we are analyzing, the performance bugs, and the process by which you identified the bugs.

Thus, you are welcome to work in teams or with an AI to help you learn the tools, read the code, and do the work. ***However, if you lean too hard on your teammates, AI, or other crutches, you may find yourself not learning much and bombing the quiz.***

## 5. Turn-in
That said, homework assignments are still worth $10\%$ of your overall grade. To submit this assignment, run the `benchmark` program with `--submission-benchmark`, then push your commit (with the generated `.benchmarks` file) to the GitHub Classroom repository from which you cloned.

_A note on cheating:_ While there is an autograder and you may want to find ways to work around it, **keep in mind** that:
1. We will be verifying performance metrics of the code you submit on the `nodes` server.
2. $10\%$ of your grade is not enough to hold you up if you can't perform on the quizzes because you cheated on the homework assignments.
