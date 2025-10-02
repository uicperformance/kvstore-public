
In this second assignment, we are starting with a solution for hw1, and extending it slightly to start supporting a multi-threaded server. Moreover, some of the default benchmark parameters have been changed. To see the specific changes, try using ``git diff hw1``. 


## 1. Understanding program behavior

By default, the hw2 code provided launches a new server thread for each incoming client connection. The ``--singlethread`` command line parameter overrides this. 

- [ ] run the benchmark with and without the --singlethread server parameter, and observe how this affects performance.
- [ ] run the benchmark with the --singlethread server parameter. Measure the time spent waiting in system calls using ``strace -cfw`` on the server process and the ``--exitcode`` parameter to both programs. 
- [ ] run the benchmark without the ``--singlethread`` parameter on the server. Measure the time spent waiting in system calls using ``strace -cfw`` and the ``--exitcode`` parameter to both programs. What differences do you notice?
- [ ] run the same tests, but with ``time`` on the server process, instead of strace. What do the ``real``, ``user`` and ``sys`` lines mean in the context of a multi-threaded program, or a single threaded program? Can ``user`` or ``sys`` ever show more seconds than what actually passed, when? What about ``real``?

## 2. Fine grained synchronization

Create a new branch in your git repository, using ``git checkout -b finegrained``. The template code uses a very coarse-grained synchronization pattern. There is a single lock, and it's held for a long time. Leave the map as an ``Arc<Mutex<TreeMap<String, String>>>``, but update the server logic to hold the lock only for as long as necessary. 

- [ ] how does this change affect performance? Is frequently acquiring and releasing the lock introducing too much extra overhead to be worthwhile, or does the improved concurrency make up for the higher cost? Observe the benchmark performance, and try a multi-threaded server run with ``strace -cfw`` to see how time spent waiting changed. Note: ``strace`` isn't free - it can substantially slow down the observed program. 
- [ ] the server doesn't do very much else than accessing the map, for which the lock is anyway required. Assuming the server has enough cores available, you should be seeing a nice speedup, but what explains the speedup you're seeing? If you ``htop``, are the extra cores working, or mostly waiting?

## 3. Readers-Writer lock

Change the fine-grained code to use an RwLock (Readers-Writer lock) instead of a Mutex to protect the map. RwLock has a ``read()`` and a ``write()`` function instead of a ``lock()`` function. Multiple readers can safely lock the ``read()`` side of the lock at the same time, while writers can't share the lock with either readers or other writers. 

- [ ] how does using a Readers-Writer lock change the performance of your fine-grained locking program?

Commit this fine-grained, rwlock version, then check out the coarse grained version (hw2 branch) and introduce the RwLock there too. 

- [ ] Which one is faster: fine-grained with RwLock or coarse-grained with RwLock? What about fine-grained with Mutex? How do you explain their relative performance? 
- [ ] Try using ``strace -c futex`` as well as ``-cw`` to see how the number of, and time spent on (and waiting on), synchronization-related system calls compares. 

## 4. More efficient batches

We'll use the fine-grained, readers-writer lock going forward for now. If you run this version of the server with ``time``, you'll find that it now spends most of its time in ``sys``, meaning processing system calls. 

- [ ] use strace -c on the server to count how many system calls are being made. Reduce this by sending all the responses of a batch using a single write_all call. You'll need to accumulate the responses in a String first. 

- [ ] Group-sending responses dramatically reduces the number of system calls, however a lot of server time is still spent in the kernel. Use ``perf record`` and ``perf report`` to see how the time is being spent. The terms ``skb`` and ``sk_buff`` stand for a socket buffer: memory allocated for a single incoming or outgoing network packet.

- [ ] Similarly update the benchmark program to also send each batch using a single ``write_all`` call. How does this affect the server's performance? Have another look at ``perf record``/``perf report`` to see how the distribution of time spent in different functions changed.

## 5. Evaluation

The evaluation for this assignment will be a paper quiz or an in-class computer exam, using your own computer.  The quiz or computer exam will cover material from hw1 and hw2, and the lectures leading up to it. 
