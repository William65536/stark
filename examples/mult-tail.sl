; SPDX-FileCopyrightText: © 2021 Matthew Rothlisberger
; SPDX-License-Identifier: AGPL-3.0-only

; Project Euler Problem 1 Solution
; using tail recursion

; run time, late March 2021: >70 seconds
; run time, April 13 2021: ~4 seconds
; run time, April 21 2021: ~3.3 seconds
; run time, October 10 2022: ~0.02 seconds
; run time, April 23 2023: ~0.09 seconds
; run time, July 26 2023: ~1.7 seconds

(def mult (fn [max acc cur]
              (if (= cur max)
                  acc
                  (do (if (= (mod cur 3) 0)
                          (set acc (+ acc cur))
                          (if (= (mod cur 5) 0)
                              (set acc (+ acc cur))
                              ()))
                      (mult max acc (+ cur 1))))))
(mult 1000 0 1)
