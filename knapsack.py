"""经典的背包问题算法"""


def knapSack(capacity: int, weights: list[int], values: list[int]) -> int:
    """
    0-1背包问题（动态规划）

    Args:
        capacity: 背包容量
        weights: 物品重量列表
        values: 物品价值列表

    Returns:
        能装入的最大价值
    """
    n = len(weights)
    dp = [[0] * (capacity + 1) for _ in range(n + 1)]

    for i in range(1, n + 1):
        for w in range(capacity + 1):
            if weights[i - 1] <= w:
                dp[i][w] = max(dp[i - 1][w], dp[i - 1][w - weights[i - 1]] + values[i - 1])
            else:
                dp[i][w] = dp[i - 1][w]

    return dp[n][capacity]


def knapSack_1D(capacity: int, weights: list[int], values: list[int]) -> int:
    """
    0-1背包问题（一维空间优化）

    Args:
        capacity: 背包容量
        weights: 物品重量列表
        values: 物品价值列表

    Returns:
        能装入的最大价值
    """
    dp = [0] * (capacity + 1)

    for i in range(len(weights)):
        for w in range(capacity, weights[i] - 1, -1):
            dp[w] = max(dp[w], dp[w - weights[i]] + values[i])

    return dp[capacity]


def bounded_knapSack(capacity: int, weights: list[int], values: list[int], counts: list[int]) -> int:
    """
    多重背包问题（每种物品有数量限制）

    Args:
        capacity: 背包容量
        weights: 物品重量列表
        values: 物品价值列表
        counts: 每种物品的数量

    Returns:
        能装入的最大价值
    """
    dp = [0] * (capacity + 1)

    for i in range(len(weights)):
        for _ in range(counts[i]):
            for w in range(capacity, weights[i] - 1, -1):
                dp[w] = max(dp[w], dp[w - weights[i]] + values[i])

    return dp[capacity]


def unbounded_knapSack(capacity: int, weights: list[int], values: list[int]) -> int:
    """
    完全背包问题（每种物品无限可选）

    Args:
        capacity: 背包容量
        weights: 物品重量列表
        values: 物品价值列表

    Returns:
        能装入的最大价值
    """
    dp = [0] * (capacity + 1)

    for w in range(capacity + 1):
        for i in range(len(weights)):
            if weights[i] <= w:
                dp[w] = max(dp[w], dp[w - weights[i]] + values[i])

    return dp[capacity]


def fractional_knapSack(capacity: int, weights: list[int], values: list[int]) -> float:
    """
    分数背包问题（物品可以分割）

    Args:
        capacity: 背包容量
        weights: 物品重量列表
        values: 物品价值列表

    Returns:
        能装入的最大价值
    """
    n = len(weights)
    items = [(i, values[i] / weights[i], weights[i], values[i]) for i in range(n)]
    items.sort(key=lambda x: x[1], reverse=True)

    total_value = 0.0
    remaining_capacity = capacity

    for _, ratio, weight, value in items:
        if remaining_capacity >= weight:
            total_value += value
            remaining_capacity -= weight
        else:
            total_value += ratio * remaining_capacity
            break

    return total_value


def knapSack_with_items(capacity: int, weights: list[int], values: list[int]) -> tuple[int, list[int]]:
    """
    0-1背包问题（记录选择的物品）

    Returns:
        (最大价值, 被选中的物品索引列表)
    """
    n = len(weights)
    dp = [[0] * (capacity + 1) for _ in range(n + 1)]

    for i in range(1, n + 1):
        for w in range(capacity + 1):
            if weights[i - 1] <= w:
                dp[i][w] = max(dp[i - 1][w], dp[i - 1][w - weights[i - 1]] + values[i - 1])
            else:
                dp[i][w] = dp[i - 1][w]

    # 回溯找出选择的物品
    selected = []
    w = capacity
    for i in range(n, 0, -1):
        if dp[i][w] != dp[i - 1][w]:
            selected.append(i - 1)
            w -= weights[i - 1]

    return dp[n][capacity], selected[::-1]


if __name__ == "__main__":
    # 测试数据
    weights = [2, 3, 4, 5]
    values = [3, 4, 5, 6]
    capacity = 8

    print("=" * 50)
    print("0-1背包问题")
    print(f"物品重量: {weights}")
    print(f"物品价值: {values}")
    print(f"背包容量: {capacity}")
    print(f"最大价值(二维DP): {knapSack(capacity, weights, values)}")
    print(f"最大价值(一维DP): {knapSack_1D(capacity, weights, values)}")

    max_val, items = knapSack_with_items(capacity, weights, values)
    print(f"选中的物品索引: {items}, 最大价值: {max_val}")

    print("=" * 50)
    print("完全背包问题(无限数量)")
    print(f"最大价值: {unbounded_knapSack(capacity, weights, values)}")

    print("=" * 50)
    print("分数背包问题(可分割)")
    print(f"最大价值: {fractional_knapSack(float(capacity), weights, values):.2f}")

    print("=" * 50)
    print("多重背包问题(每种物品限2个)")
    counts = [2, 2, 2, 2]
    print(f"最大价值: {bounded_knapSack(capacity, weights, values, counts)}")
