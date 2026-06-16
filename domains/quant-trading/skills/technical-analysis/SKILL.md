---
name: technical-analysis
domain: quant-trading
version: 1
trigger_patterns:
  - "chart patterns"
  - "technical indicators"
  - "price action"
  - "support and resistance"
  - "market structure"
applicable_agents:
  - market-analyst
  - strategy-designer
---
# Technical Analysis

## Steps
1. Identify market structure: trend direction, support/resistance levels, swing highs/lows
2. Apply indicators: trend (MA, MACD), momentum (RSI, Stochastic), volatility (Bollinger Bands, ATR)
3. Recognize chart patterns: head and shoulders, double top/bottom, triangles, flags
4. Analyze volume: confirm breakouts, divergence signals, accumulation/distribution
5. Check multiple timeframes: higher timeframe for trend, lower for entry timing
6. Combine confluence: align signals from 2-3 independent indicators before acting

## Examples
- Trend following: price above 200-day MA = uptrend; RSI > 70 = overbought but trend is up
- Reversal pattern: double top at resistance with bearish RSI divergence = potential short
- Breakout: ascending triangle with volume spike = bullish continuation

## Anti-patterns
- Indicator overload (paralysis by analysis — too many conflicting signals)
- Ignoring higher timeframe context (counter-trend trades against the daily trend)
- Rearview mirror bias (fitting patterns after the move has happened)
- Mechanical indicator use without understanding market context
