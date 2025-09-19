
In this fourth assignment, we are starting with the same code as in hw3, but instead of analyzing what's wrong, we implement some fixes. 

They key problem we observed in hw3 was that the server keeps saving the database every time there is an update. 
Program crash persistence is a reasonable requirement for a database. Sometimes, persistence through kernel crash or power failure are also required, but this is beyond the scope of this assignment. 

## 1. Batch-wise persistence

Since the socket protocol of our key-value store supports batching requests, we may reasonably relax the persistence requirement from _every request_ to _every batch_. After all, the client won't know the difference since we are only sending the responses once the entire batch is completed. 

Change the server to save the database to disk only once per batch, and only if there were any changes. 

- [ ] Try running this version with different batch sizes configured in the benchmark client, and see how this impacts performance. 

## 2. Add Write-Ahead Logging

Performance is still quite poor, however, and very large batches may not be realistic in many real applications. To do better, we can implement write-ahead logging: instead of frequently writing out the entire database to disk, simply append all the (successful) original requests to a text file instead. 

On server startup, the template code already first re-does all the requests in the log, before serving new clients. 

- [ ] on each update (set / remove), append the request to a log file (use the file name specified in the ``--logfile`` argument). Then process the request as usual, but don't save the table to disk. 
- [ ] how does this impact the number of write system calls? What about the duration of each write system call?
- [ ] for even better performance, write the log to disk only once per batch*. 
- [ ] many database systems and file servers use a separate, faster device for logging. Try putting the log on our fast "device" ``/tmp/``. 

** Note: this all works nicely for application crashes. In the event of power failure, updates may still be lost. To prevent that, you'll need to use the ``fsync`` system call, or close the file. These are both fairly expensive, and out of scope for this assignment. **

## 3. Avoid long startup times with periodic snapshots

Keeping all updates in a log will eventually lead to extremely long startup times. Consider a database that has been serving a million requests per second, for the past day.... 

A better solution is to use write-ahead logging until the log exceeds a certain length, then write out the full database to disk, and zero out the log. 
To zero out an open log file, use both Seek::rewind(), and File::set_len(0) together. 

- [ ] Add a snapshot interval command line argument to the server.
- [ ] Observe the throughput achieved for snapshot intervals ranging from 10 requests to 10000 requests
- [ ] Observe the mean and tail latency for snapshot intervals ranging from 10 requests to 10000 requests
- [ ] What do you notice about the relationship between mean latency and throughput? 
- [ ] Is there a similar relationship between tail latency and throughput?
- [ ] Measure mean/tail latency over batch size. 

## 4. Evaluation

The evaluation for this assignment will be an in-class paper quiz, with no computer access. The quiz will cover material up to hw3, and material leading up to it, though the focus will be on recent material. 
