# may-todos

R&D of may_minihttp. Not a realistic setup.

## Benchmark

- Apple M1 Max
- 32GB RAM
- Small data-set (10 rows)
- PostgreSQL in docker

YMMV:

```shell
may-todos main ➜ wrk -t10 -c100 -d30s http://localhost:8080/stories
Running 30s test @ http://localhost:8080/stories
  10 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.47ms  608.64us  15.10ms   86.81%
    Req/Sec     6.15k     1.06k   29.38k    76.81%
  1836527 requests in 30.10s, 669.05MB read
Requests/sec:  61017.02
Transfer/sec:     22.23MB
```

## References

See [here](https://github.com/Xudong-Huang/may_minihttp/blob/master/examples/techempower.rs) for the reference example used.
