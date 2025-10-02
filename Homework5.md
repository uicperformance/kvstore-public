
In this fifth assignment, we are starting with an extended hw4 solution as a template. 

The server has been improved to support easily switching between different key types: String, [u8;N] and usize. 
It runs in mem-only mode by default (nothing is written to disk), reads large request batches more efficiently, and has a new "CLEAR" command, allowing the benchmark program to remotely clear the entire database, rather than have to manually restart it.

Other settings have changed as well: the benchmark runs single thread, larger batches, and uses keys of a smaller range. 
Most of these changes serve to shift the performance emphasis of the program: not much has changed in the actual operation of the server. 

Below, you'll study several aspects of the server's performance in this new context. In the next assignment, we'll 
use what you learned to try to improve the performance of this server further. 

As always, keep in mind that the point of this assignment is not to "finish" it, but to have an opportunity to practice 
relevant tools and skills in a realistic (but friendly) setting. You're highly encouraged to discuss the questions with your classmates, and exchange ideas for how to best diagnose the performance of this program. 
 
## 1. What are we spending time on now?

Running the benchmark program with default parameters, you'll now find that the server reaches approximately 1 million operations per second. If you run the server with ``time`` and an exit code, you'll find that about half the time is spent in user space. The waiting is primarily explained by the benchmark client and the server both being single-threaded and "taking turns" to do work: the benchmark client waits until it gets a response, then prepares the next set of requests. And vice versa. We'll address that in a later assignment.

Focus on the time spent running user code, and try to find the answers to these questions:

- what's calling  __memcmp_avx2_movbe_rtm and why?
- what is it about TreeMap::insert and TreeMap::get that's taking the most time?
- what's this memchr_aligned that takes up 8% of our cycles?

You may find ``perf record --call-graph=dwarf`` handy for some of these questions, as well as ``gdb``. 
You'll get a fast, and quite usable ``perf report`` when recording with call-graph if you pass the parameters ``--no-inline --no-children``. 

## 2. Now try it with all reads

Run the same experiment, but with --rw-ratio 100. It's no big surprise that TreeMap::insert disappeared, but have a
closer look at the memory handling. It's a big chunk of the time. 

What is causing all this memory allocation overhead when all we are doing is reading?
Dig around a little in the ``--call-graph``, but let's try another exploratory experiment to see what more we can learn.

## 3. Now try it with a smaller working set

Run the same experiment, but pass ``--key-range 100``, to use a very small working set. This way, we can be fairly certain that the entire TreeMap fits in the L1 cache of the server. 

Make another fresh perf report, and see how this changed things. 

- Did a smaller working set help reduce the cost of memory management? 

The call graph report will show you only the calling function, not the line number, which is a little coarse-grained for this situation. 

- Try to work out what exactly is causing all these memory allocations and frees.

You may find ``gdb`` useful here, perhaps using a breakpoint and the ``command`` trick shown in class.

## 4. Switch to fixed-size keys and values

Modify ``server.rs`` to set the KeyType to FixedSize instead of String. This results in keys being treated as fixed 32-byte arrays instead of variable-size String. 

Finally, modify ``server.rs`` to set the ValueType to FixedSize as well. You will get a runtime assertion failure when you try to run the benchmark, but if you set the benchmark value-size to 32 bytes, it'll run. This should now give you a relatively good clue as to where part of the memory allocations came from. 

- Why does using a FixedSize value result in less time spent on memory allocations?
- There are many memory allocations left. What's behind those?
- Other than reducing memory allocations, did FixedSize eliminate any other work vs. String?

## 5. Try Integer keys

Change ``server.rs`` again to use Integer keys instead of FixedSize, leaving ValueType as FixedSize. 

- Where did we save the most time going to Integer keys? Try to nail down the specific operation(s) that were sped up.

## Turn-in

A brief questionnaire based on the questions above will be posted on gradescope, to serve as the turn-in for this assignment. 

## In-class evaluation

After the due date, we will have an in class quiz or computer exam, covering this assignment and all class content leading up to it.
