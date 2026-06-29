+++
title = "On making good out-of-process APIs"
draft = true
+++

# On making good out-of-process APIs

### APIs and Sadness

Application Programming Interface. This is an acronym. Better known to humans who deal with computers as an API. And like all good acronyms, it has become overused and had its meaning muddied, lol.

We are going to say that an API is just a boundary in a system where information is exchanged. Like 2 tin cans connected by a string, or one of those shape hole things [from that one video](https://www.youtube.com/watch?v=6pDH66X3ClA). Actually this video is kind of a good example of why API design is important. If all of the shapes fit in the square hole, people will be sad.

We want people to be happy and not sad and therefore we need to have good api design so that all the shapes go into the expected holes. Make sense? Let's begin.


### Defining API

Computer programs generally run inside of an operating system and all the popular operating systems separate these programs into processes, where one process should not be able to read the memory of another process.

This basically gives us 2 categories of "boundaries" or APIs: boundaries that exist inside of a process and boundaries that exist in between processes. Lets call these in-process API and out-of-process API respectively.

Let's define an out-of-process API as _an interface where transport is expensive because we need to do serialization/deserialization to cross a boundary because we cant share memory_


Here are a bunch more acronyms that fall into our out-of-process category:

* IPC (e.g. Unix pipe)
* REST
* gRPC
* GQL

### Tools

Since the boundary is expensive to cross we need to make sure that we are bringing the correct data with us when we pass in our request and get back our response. We have 2 really good tools for doing this `filter` and `sort`.

Filter lets us throw out data that we dont care about like emails that are older than 7 days.

Sort lets us put responses in a predictable order.

Lets set up an example.

_Process A has some data_
```
[1,2,3,4,5,6,7,8,9,10]
```

our program in _Process B_ needs to display all of the even numbers that exist in _Process A_ in descending order but we can only receive 1 number at a time.

how do we solve this?


### Reinventing the Wheel

Well first lets try to define the simplest posible interface. We will use some Rust pseudocode for this but the concept applies to any language. All you need to know is that the left side of the arrow is the _request_ and the right side is the _response_

```rust
trait API {
  fn get() -> usize
}
```

Now if _B_ calls `API::get()` then _A_ will probably return 1 since its the first item in the list. That's a failure because 1 is not even per our requirements. So lets reach for some filter logic.


If we change the interface to be

```rust
enum Num {
  Even,
  Odd
}

trait API {
  fn get(filter: Num) -> usize
}
```

Now _B_ can call `API::get(Num::Even)` and _A_ will probably return 2 provided _A_'s implementation of `is_even` is correct. This is better because the number is now even but its still wrong because our requirements state that we must display the numbers in descending order and 10 is greater than 2.

