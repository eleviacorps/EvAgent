---
name: market-regime-detection
domain: quant-trading
version: 1
trigger_patterns:
  - "market regime"
  - "bull/bear market"
  - "market cycle"
  - "regime change"
  - "trending vs ranging"
applicable_agents:
  - market-analyst
  - strategy-designer
  - risk-manager
---
# Market Regime Detection

## Steps
1. Determine trend state: use 200-day MA slope, ADX, or MACD to classify bullish/bearish/sideways
2. Measure volatility: VIX level, ATR, Bollinger Band width — low vs high vol regime
3. Assess market breadth: advancing/declining ratio, new highs/lows, % stocks above 50-day MA
4. Identify cycle phase: expansion (rising GDP, low unemployment), peak, contraction, trough
5. Detect regime transitions: volatility breakouts, trendline breaks, volume confirmation
6. Adapt strategy: trend-following in trending markets, mean-reversion in ranging markets

## Examples
- Bull trend + low vol = trend-following strategies work best
- Bear trend + high vol = mean-reversion and hedging strategies
- Sideways + low vol = options selling (theta capture) and pairs trading
- Regime change signal: VIX spike + 200-day MA cross + breadth deterioration

## Anti-patterns
- Using the same strategy in all market conditions
- Calling every pullback a regime change
- Ignoring regime context when evaluating strategy performance
- Lagging indicators that confirm the regime after it's already ended
