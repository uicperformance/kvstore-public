
In this sixth assignment, we start with a program very similar to Homework 5. 
As we figured out in Homework 5, much of our time is spent allocating and accessing memory. 
In particular, much time is spent allocating and freeing Strings, both by the TreeMap itself, and by the server.

## 1. An improved get() - Introduction

The homework 6 template provides a slightly improved implementation of the ``get()`` function in TreeMap. It used to take the form:

``pub fn get(&self, key: &K) -> Option<V>``

In other words, the hw5 template took a borrow of the KeyType, in this case a String. This used to force the caller to create a KeyType (e.g. String), if they don't already have one. In our case, all we had was a &str from the incoming command, so we created a KeyType from the &str, and passed that to the ``get()`` function. In the case of String, this means a temporary String is allocated, and then freed once ``get()`` returns. 

Moreover, it returns an ``Option<V>``, which means it creates and returns a ``ValueType``. This means different things for different types, but for String, it means a new heap allocation is made, and the contents are copied. For an Integer, it's just a matter of passing an integer value around (another form of copying, but much less cumbersome). This ``Option<String>`` is used in the ``match`` statement that uses the return value of ``get()`` in the server, and then dropped. So, one more allocation. 

In hw6, it looks like this:

``pub fn get<'a, B: std::cmp::Ord+?Sized>(&'a self, key: &B) -> Option<&'a V> where K: Borrow<B>``

Here, ``B: std::cmp::Ord+?Sized>(&'a self, key: &B)`` and ``K: Borrow<B>`` introduces a new generic type B. B is a type such that K implements ``Borrow<B>``. That is, we can get a &B out of a KeyType if we ask for it.
Moreover, B supports comparison, and we don't require it to have a known size. By default, types must have a known size, so the ``?Sized`` says we don't care. This is handy, as ``str`` does not implement ``Sized``. 

Moreover, it returns ``Option<&'a V>`` instead of ``Option<V>``. That is, we return a borrow of a value, instead of the value itself. The borrowed value is the value that is stored in the tree - after all, we don't need two copies of it. However, this could be risky if we were using pointers: what how do we know the pointer is still good by the time we use it? 

Here, the Rust compiler offers a lending hand. The notation ``&'a self`` says "given a read-only borrow of, which lives for a duration we will call ``a``, this function returns a borrow of a value which will also live for the duration ``a``." That is, we say the reference is good for as long as our borrow of the tree map is good. The compiler checks both that the implementation of ``get()`` can actually make this promise, and that the caller of ``get()`` doesn't try to keep the borrow beyond the duration of the TreeMap borrow. It'll also check that we don't try to mutate the TreeMap until we are done using the returned &V, which otherwise might have resulted in a dangling borrow. This is a lot--take a moment to read about borrow lifetime (https://doc.rust-lang.org/rust-by-example/scope/lifetime.html).

The end result, however, is that get neither requires a String key, nor returns a String value, making it possible to write more efficient server code. The provided server code, however, is not taking advantage of this. Instead,
it is written to mimic the hw5 design as closely as possible. Here, the syntax ``Into::<KeyType>::into()`` is the same as ``.into()``, except it is explicit about the target type. Because the TreeMap no longer takes a
specific type, but can accept any type that meets the criteria, the compiler can't decide on its own which of potentially infinite different types to use. Instead, we have to tell it. 

First off, try running the default benchmark and the default server, both with ``--release``, to get a baseline performance reference. It's probably around 1 million operations/s now. 

## 2. Make use of the improved get()

We'll focus on ``get`` performance for now. You can use ``--rw-ratio 100`` to have the benchmark issue only ``GET`` commands during performance testing. Use ``perf`` with and without ``--call-graph`` as in ``hw5``, to see how much time is spent on allocating and freeing memory. Then, modify ``server.rs`` to make better use of the new ``get()`` design (should be a one-line change), and observe the difference in performance, and in time spent on memory management. 

Try to work out how much time was spent on the key, and how much on the value. Then, notice how much time we _still_ spend on allocation after eliminating these efficiencies.

## 3. Track down and kill more memory allocations

You'll find several more sources of memory allocations in the handling of ``get`` requests. Use ``perf record --call-graph`` and ``gdb`` to track down exactly what is making them, and eliminate them. 

We'll not provide too much guidance here, except to say that the solution spends much less than half a percent of its time on heap management. 

In at least a couple of cases, the most convenient and general way of doing things, using powerful abstractions and functions, tends to require memory allocation (and hide them rather well). 

However, when you are able to make some simplifying assumptions, and willing to spend a few lines of extra code writing out your own (but perhaps less elegant-looking) solution, you can get away without using heap memory. 

When pursuing optimizations like this, always remember these words of wisdom attributed to Donald Knuth: "_Premature optimization is the root of all evil_". Another way to put it is this: let the profiler guide your efforts. 
Don't spend your precious time and attention optimizing code that hardly ever runs. 

The solution achieves almost 2 million operations per second. 

## 4. Turn in 

**Jacob, please add**

## 5. Evaluation

In addition to the turn-in, we will have an in-class evaluation, either a quiz or a computer exam, on the assignment due date.  







