---
name: portfolio-construction
domain: quant-trading
version: 1
trigger_patterns:
  - "portfolio allocation"
  - "asset allocation"
  - "modern portfolio theory"
  - "portfolio optimization"
  - "rebalancing"
applicable_agents:
  - risk-manager
  - strategy-designer
---
# Portfolio Construction

## Steps
1. Define investment objectives: return target, risk tolerance, time horizon, liquidity needs
2. Choose asset allocation: equities, fixed income, commodities, alternatives based on objectives
3. Apply portfolio theory: mean-variance optimization, risk parity, or equal-weight approaches
4. Diversify: across assets, sectors, geographies, and strategies
5. Implement sizing: risk budgeting — allocate risk (volatility), not just capital
6. Rebalance periodically: calendar-based (quarterly) or threshold-based (5% drift)
7. Monitor and adjust: track tracking error, factor exposure, performance attribution

## Examples
- 60/40 portfolio: 60% equities (SPY) + 40% bonds (AGG), rebalance annually
- Risk parity: allocate so each asset contributes equal portfolio risk (e.g., levered bonds, less equities)
- All-weather: 30% stocks, 40% long-term bonds, 15% intermediate bonds, 7.5% gold, 7.5% commodities

## Anti-patterns
- Over-concentration in familiar assets (home country bias, company stock)
- Rebalancing too frequently (transaction costs eat returns)
- Ignoring correlations during tail events (they converge to 1)
- Chasing past performance (buying high, selling low)
