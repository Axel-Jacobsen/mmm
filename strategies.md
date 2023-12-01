# Strategies, yo!


## copy-copy-copy

- Take top `n` active traders
    - define by wealth, wealth plus some rate of recent weath accrual?
- Watch their trades, and on trades on "big enough" markets...
    - defined by some constant volume? defined by whether the market pct changes?
- ... copy their trades

Pros:
    - just copy the best `n` traders
    - easy to implement
Cons:
    - maybe it'd mess with their trades and people wouldn't like it
    - I'd be getting in late on every trade

## anti-copy-copy-copy

- Take bottom `n` active traders
    - define by wealth, wealth plus some rate of recent weath loss?
- Watch their trades, and on trades on "big enough" markets...
    - defined by some constant volume? defined by whether the market pct changes?
- ... inverse their strategy

Pros:
    - easy to implement
Cons:
    - definitely mean?

## market making

- Figure out how to make markets
- https://www.informs-sim.org/wsc15papers/027.pdf

Pros:
    - would be interesting to learn
    - could be interesting to see how strategy changes compared to stock market, compared to different types of manifold markets, etc
Cons:
    - would be hard maybe?

## +EV? (the "posev" strategy)

- calculate EV of each market, and if it's positive, bet

Pros:
    - easy to implement
Cons:
    - risky? would have to think carefully about risk
