#pragma once
#include <string>
#include <cmath>
#include <cctype>

class MathParser {
    const char* expr;
    double currentX;

    double parseFactor() {
        while (*expr == ' ') expr++;
        if (*expr == '+' || *expr == '-') {
            bool neg = *expr == '-';
            expr++;
            double val = parseFactor();
            return neg ? -val : val;
        }
        if (*expr == '(') {
            expr++;
            double val = parseExpression();
            if (*expr == ')') expr++;
            return val;
        }
        if (isalpha(*expr)) {
            std::string func;
            while (isalpha(*expr)) func += *expr++;
            if (func == "x") return currentX;
            if (*expr == '(') {
                expr++;
                double val = parseExpression();
                if (*expr == ')') expr++;
                if (func == "sin") return std::sin(val);
                if (func == "cos") return std::cos(val);
                if (func == "tan") return std::tan(val);
            }
        }
        double val = 0;
        while (isdigit(*expr) || *expr == '.') {
            if (*expr == '.') {
                expr++;
                double frac = 1.0, dec = 0.0;
                while (isdigit(*expr)) {
                    frac /= 10.0;
                    dec += (*expr++ - '0') * frac;
                }
                val += dec;
            }
            else {
                val = val * 10.0 + (*expr++ - '0');
            }
        }
        return val;
    }

    double parseTerm() {
        double val = parseFactor();
        while (true) {
            while (*expr == ' ') expr++;
            if (*expr == '*') { expr++; val *= parseFactor(); }
            else if (*expr == '/') { expr++; val /= parseFactor(); }
            else if (*expr == '^') { expr++; val = std::pow(val, parseFactor()); }
            else break;
        }
        return val;
    }

public:
    double parseExpression() {
        double val = parseTerm();
        while (true) {
            while (*expr == ' ') expr++;
            if (*expr == '+') { expr++; val += parseTerm(); }
            else if (*expr == '-') { expr++; val -= parseTerm(); }
            else break;
        }
        return val;
    }

    double Evaluate(const std::string& expression, double xValue) {
        expr = expression.c_str();
        currentX = xValue;
        return parseExpression();
    }
};