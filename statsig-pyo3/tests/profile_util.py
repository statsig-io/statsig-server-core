import time

def profile(action, iterations=100_000):
    overall_start = time.perf_counter()
    durations = []

    value = None

    for _ in range(iterations):
        start = time.perf_counter()
        value = action()
        end = time.perf_counter()
        durations.append((end - start) * 1000)

    overall_end = time.perf_counter()
    overall = (overall_end - overall_start) * 1000
    durations.sort()

    p99_index = int(iterations * 0.99)

    p99 = durations[p99_index]
    min = durations[0]
    max = durations[-1]

    return {
        "value": value,
        "overall": overall,
        "p99": p99,
        "min": min,
        "max": max,
    }