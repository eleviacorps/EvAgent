---
name: risk-calculation
domain: quant-trading
version: 1
trigger_patterns:
  - "position sizing"
  - "Value at Risk"
  - "VaR"
  - "drawdown analysis"
  - "risk metrics"
  - "portfolio risk"
applicable_agents:
  - risk-manager
  - strategy-designer
---
# Risk Calculation

## Steps
1. Calculate position size: use Kelly Criterion, fixed fractional, or volatility-based sizing
2. Compute Value at Risk (VaR): parametric (variance-covariance), historical, or Monte Carlo methods
3. Measure drawdown: peak-to-trough decline, max drawdown, average drawdown, recovery time
4. Calculate risk-adjusted returns: Sharpe, Sortino, Calmar, and Information ratios
5. Assess correlation and diversification: portfolio beta, correlation matrix, concentration risk
6. Stress test: scenario analysis (2008, COVID flash crash), what-if simulations

## Examples
- Position sizing: 2% risk per trade (if stop-loss is 5%, position = 2/5 = 40% of capital)
- VaR calculation: "95% daily VaR of $10K" means 95% of days loss ≤ $10K
- Kelly formula: f* = (bp - q) / b, where b = odds, p = win probability, q = loss probability

## Anti-patterns
- Over-leveraging based on backtest results (real markets have drawdowns)
- Ignoring tail risk (VaR doesn't capture extreme events beyond the confidence level)
- Using VaR alone without stress testing
- Assuming correlations stay stable during crises (they converge to 1)
