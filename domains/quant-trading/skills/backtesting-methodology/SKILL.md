---
name: backtesting-methodology
domain: quant-trading
version: 1
trigger_patterns:
  - "backtest strategy"
  - "backtesting methodology"
  - "strategy evaluation"
  - "historical simulation"
applicable_agents:
  - strategy-designer
  - risk-manager
---
# Backtesting Methodology

## Steps
1. Define strategy rules precisely: entry, exit, position sizing, stop-loss, take-profit
2. Select historical data: adequate period (5+ years), multiple market regimes, appropriate granularity
3. Handle look-ahead bias: use only information available at the time of the trade
4. Account for transaction costs: commissions, slippage, spread, market impact
5. Run out-of-sample tests: split data into training/validation/test sets
6. Evaluate key metrics: Sharpe ratio, max drawdown, win rate, profit factor, CAGR
7. Perform robustness checks: Monte Carlo simulation, parameter sensitivity, walk-forward analysis

## Examples
- Moving average crossover: test on 10yr data, reserve last 2yr for out-of-sample validation
- Mean reversion: add 10bps slippage per trade, test with different holding periods
- Machine learning strategy: cross-validate time series, avoid data leakage from future normalization

## Anti-patterns
- Overfitting to historical data (too many parameters, too few trades)
- Survivorship bias (using only stocks that still exist today)
- Ignoring transaction costs (looks great in backtest, bleeds in live trading)
- Peeking (adjusting strategy based on test set performance)
