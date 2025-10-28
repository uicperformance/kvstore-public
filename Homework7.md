In this 7th homework assignment, we start with a solution to Homework 6, which eliminates the memory allocation cost of GET requests.
Memory allocation work is unavoidable when inserting or deleting key-value pairs in the database, so we'll leave that alone for now.

Instead, we turn our focus to the high cost of finding an element in the tree. The benchmark generates keys uniformly at random, resulting in a relatively well-balanced binary tree. For 1,000 entries, the depth is around 20, for 1,000,000 entries about 50, within a factor 3 of optimal. Not to shabby for a tree that's not even trying to stay balanced. 

The hw7 template includes an alternative data structure, called a BTree. Compared to a binary tree, the biggest difference in a BTree is that it has a larger out-degree than 2. While BTrees offer particular benefits for external storage (on disk), the provided implementation is an in-memory BTree. In fact, it doesn't even support writing to disk at all. Thus, our focus is on in-memory performance alone.

The template provides facilities for easily changing between the TreeMap and BTree implementations (the MapType definition in server.rs). 
By default, it is built with the TreeMap as usual. To build the server with a BTree instead, simply pass ``--features btree`` to ``cargo build`` or ``cargo run``. Always build with ``--release`` when analyzing performance. 

## Summary statistics with perf stat

The template comes with the TreeMap and the BTree version configured to use String keys. Run both versions with ``perf stat`` and analyze the results carefully. You'll note that the server prints the type: use this to double-check that you don't mix up your measurements. 
It also prints the size (number of elements) and depth (number of levels) of the tree. Naturally, a BTree can make do with many fewer levels than a binary tree. Use the default benchmark settings, but make sure you add the ``--exit-code`` option to have the server exit after the benchmark completes. 

- The BTree version has lower throughput and takes longer to finish than the TreeMap. Based on the perf stat output, what likely explains this? 
- That said, the BTree version runs faster in one sense: perf stat reports more instructions per cycle. What could explain that? Think about the cachestrees program demonstrated in class.

## Analyzing cache behavior with perf stat

If you profile these programs with ``perf record``, you'll find that they spend most of their time on a small number of instructions, and essentially the same instructions. As a result, it can be difficult to tease apart the difference between our two programs using profiling. Instead, we will use 
aggregate, but detailed performance counters, and multiple runs with different arguments, to learn more about the programs. 

Run both servers with ``perf stat -e L1-dcache-loads -e L1-dcache-load-misses -e l2_rqsts.references -e l2_rqsts.miss -e instructions -e cycles`` 
instead. This measures L1 and L2 cache load requests and misses. 

- Do these new measurements agree with your hypothesis from above? 
- Based on this, why do you think the BTree version achieves more instructions per cycle?

## Learn more with a different experiment

Using the same ``perf stat`` measurement as above, try running both versions of the server, but add these arguments to the benchmark program: ``--prepopulate 100000 --key-range 100000``. The default setting results in a tree of size 1000. These settings make a considerably larger tree, 
at over 65,000 key-value pairs. 

You'll find that the throughput for both versions drops by about half. Judging by the ``perf`` output, the BTree version does a lot more work, but does it in about the same amount of time. 

- The default benchmark setting is 5 million operations. How many L2 cache misses are the two versions incurring per operation? 
- Given the operation of a BTree and a binary tree, and the reported depths of the trees, how do you account for all these misses?
- Use perf record -e l2.miss to profile the servers based on L2 misses. This tells you where in the program the L2 misses occur. How do the two versions differ in this regard?

## Dig deeper with stall cycles

Use ``perf list stall`` to see a list of events related to CPU core stalls. A stall is when the CPU can't find anything more to do except 
wait for something to finish. Since we are exploring L2 misses, add ``-e cycle_activity.stalls_l2_miss`` to the ``perf stat`` command above, and try again. 

- Note how the BTree version incurs much fewer stall cycles per L2 miss than the TreeMap version. How might speculative execution explain this?

## Switch things up with FixedSize keys

From the measurements above, we've found that the BTree version does a lot more work than the TreeMap, but is able to do complete all that work
in about the same amount of time, largely due to spending much less time waiting at L2 cache misses. 

That said, it still spends about 20\% of its time on such misses, primarily doing key comparisons. We can eliminate many L2 misses by
eliminating the pointer dereferences that come with a String key. Using a FixedSize key, the keys can be part of the tree node itself instead
of a separate heap allocation. 

Change the KeyType in server.rs to FixedSize, then run the same experiment again, with both versions of the server. You'll find that FixedSize speeds up both programs.

- The BTree version now runs quite a bit faster than the binary tree version. What explains the difference? Why did BTree gain more with FixedSize keys?
- The BTree version incurs about 4 L2 misses per tree level, per operation. How would you account for those misses? If you're drawing a blank, try increasing the FANOUT in server.rs from 16 to 64 and compare. 

## Challenge questions

- What is the ideal FANOUT and why?
- The current BTree find_index_linear starts at index 0 and scans up until it finds a key that is too large. What if we start in the middle and scan up or down based on the first comparison? Would/does that improve performance?
- 

## Turn-in

There will be a gradescope turn-in form posted.

## Evaluation

As usual, we will have an in-class quiz or computer exam at the assignment due date. 




