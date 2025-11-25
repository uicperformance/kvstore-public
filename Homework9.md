In this ninth and final homework, we are returning to multi-threaded operation. Like Homework 7, this template is based on the solution for Homework 6.

## Performance bottlenecks with varying number of server cores

Build the server with the ``btree`` feature, then run it using ``taskset -c`` tool to restrict the number of server cores. ``taskset`` does not allow you to limit the number of cores - you have to specify a specific range of cores you want the server to use. 

## Analyze the existing program

Starting with a single server core, run the benchmark client with 1, 2 and 4 threads. Keep in mind that the server launches one thread per client - with ``taskset`` restrictions in place they are simply scheduled on the same core. 

- Performance improves significantly between 1 and 2 client threads. What explains this? Try using `htop` to observe the server core as the benchmark program runs. 
- Between 2 and 4 client threads, there is essentially no further throughput improvement. Instead the reported tail latency grows substantially. How do you explain this?

Now change your server restrictions to allow it two threads instead of one. With two server threads, and two client threads, throughput is roughly the same as one server thread and two client threads: just under 2 MOPs. 

- Why does adding more server capacity not improve the reported performance? Again, consider using `htop` here. 
- End-to-end performance, but how the server spends its time did change. What function uses a larger proportion of the server's time now? 

Use `perf diff` to see the differences clearly: first, record a baseline with `perf record -o baseline.data`, then make a second recording of a modified program, say `modified.data`. Now, `perf diff baseline.data modified.data` will show you the changes, sorted by biggest difference. This works best when changing parameters (like we are doing here), rather than modifying the executable. 

Now fix the number of client threads to 32, and vary the number of server threads from 1 to 16. 

- How does performance change as you increase the number of server threads? Make a quick gnuplot of throughput vs. server threads to get a better picture. 
- For larger thread counts, the server is spending more and more time in which function?
- What is it doing in that function that's restricting server scalability?

## Try a reader-biased approach

With the default settings, the benchmark runs 100% `get` requests after initialization. Modify the server to hold on to the RwLockReadGuard returned by `read()` between individual `get` requests. For correctness, make sure you (a) release the guard at the end of each batch to avoid starving writers in other threads, and (b) drop the guard before processing any `set` requests to avoid deadlock. You may want to use a local variable `Option<RwLockReadGuard<'_,_>>` to store the guard between loop iterations. 

- How did this change the performance? Try plotting the performance before and after the change, as two separate lines. 

Now run the benchmark with `--rw-ratio 99` instead of `100`. 

- Performance is now 10 times slower. Clearly, this was caused by our holding on to the lock guard, but how do the server threads spend their time now? Have another look at `htop`, running with 16 server threads.  
- What is causing the server threads to have such low CPU utilization now? Think both about the why, and the how. 

## Stretch goal: Improve support for read-write workloads

Keeping the benchmark client at a 99% read workload, see if you can improve the server's performance further. Here are some ideas you can try, or at least consider:

- Sharding: divide the key-space into shards a power-of-two number of shards, and keep a separate BtreeMap, behind a separate ReadersWriterLock for each shard. This reduces lock contention, but increases the number of lock acquisitions required per batch.
- Log Structured Table: keep two BtreeMaps. One for old data, which is large, but read-mostly. One for new data, which is small and read-write. For each query, check the new table first, then the old. When the new table gets to be too big, acquire a write lock on the old table and insert all the entries from tho new table into the old one. 

## Turn-in

A gradescope turn-in form will be posted for a selection of the questions above. 
