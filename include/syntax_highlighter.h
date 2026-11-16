// Colors the input editor's text from the syntax analysis Rust makes
// (tree-sitter spans and errors, as JSON; see src/syntax.rs).
#pragma once

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QVariantMap>
#include <QtGui/QSyntaxHighlighter>
#include <QtGui/QTextDocument>
#include <QtQuick/QQuickTextDocument>

class SyntaxHighlighter : public QSyntaxHighlighter {
    Q_OBJECT
    Q_PROPERTY(QQuickTextDocument* document READ quickDocument WRITE setQuickDocument NOTIFY documentChanged)
    // {"spans": [[start, length, kind]], "errors": [[start, length, message]]}
    Q_PROPERTY(QString analysis READ analysis WRITE setAnalysis NOTIFY analysisChanged)
    // kind ("keyword", "string", ...) -> color
    Q_PROPERTY(QVariantMap colors READ colors WRITE setColors NOTIFY colorsChanged)
    Q_PROPERTY(QColor errorColor READ errorColor WRITE setErrorColor NOTIFY colorsChanged)
    // Language server diagnostics by severity (2 warning, 3-4 information/hint).
    Q_PROPERTY(QColor warningColor MEMBER m_warningColor NOTIFY colorsChanged)
    Q_PROPERTY(QColor infoColor MEMBER m_infoColor NOTIFY colorsChanged)

public:
    explicit SyntaxHighlighter(QObject* parent = nullptr) : QSyntaxHighlighter(parent) {}

    QQuickTextDocument* quickDocument() const { return m_quick; }
    void setQuickDocument(QQuickTextDocument* d)
    {
        if (d == m_quick) return;
        m_quick = d;
        setDocument(d ? d->textDocument() : nullptr);
        emit documentChanged();
    }
    QString analysis() const { return m_analysis; }
    void setAnalysis(const QString& json)
    {
        if (json == m_analysis) return;
        m_analysis = json;
        const QJsonObject o = QJsonDocument::fromJson(json.toUtf8()).object();
        m_spans = o.value(u"spans").toArray();
        m_errors = o.value(u"errors").toArray();
        emit analysisChanged();
        rehighlight();
    }
    QVariantMap colors() const { return m_colors; }
    void setColors(const QVariantMap& c)
    {
        m_colors = c;
        emit colorsChanged();
        rehighlight();
    }
    QColor errorColor() const { return m_errorColor; }
    void setErrorColor(const QColor& c)
    {
        m_errorColor = c;
        emit colorsChanged();
        rehighlight();
    }

signals:
    void documentChanged();
    void analysisChanged();
    void colorsChanged();

protected:
    void highlightBlock(const QString& text) override
    {
        const int base = currentBlock().position();
        const int end = base + text.size();
        auto clip = [&](int s, int len, auto&& apply) {
            const int a = qMax(s, base), b = qMin(s + len, end);
            if (b > a) apply(a - base, b - a);
        };
        for (const QJsonValue& v : m_spans) {
            const QJsonArray s = v.toArray();
            const QVariant color = m_colors.value(s.at(2).toString());
            if (!color.isValid()) continue;
            QTextCharFormat f;
            f.setForeground(color.value<QColor>());
            if (s.at(2).toString() == u"keyword") f.setFontWeight(QFont::DemiBold);
            if (s.at(2).toString() == u"comment") f.setFontItalic(true);
            clip(s.at(0).toInt(), s.at(1).toInt(), [&](int a, int n) { setFormat(a, n, f); });
        }
        for (const QJsonValue& v : m_errors) {
            const QJsonArray e = v.toArray();
            const int sev = e.size() > 3 ? e.at(3).toInt() : 1;
            const QColor col = sev <= 1 ? m_errorColor : sev == 2 ? m_warningColor : m_infoColor;
            clip(e.at(0).toInt(), qMax(1, e.at(1).toInt()), [&](int a, int n) {
                for (int i = a; i < a + n; ++i) {
                    QTextCharFormat f = format(i);
                    // (Qt Quick draws only straight underlines.)
                    f.setFontUnderline(true);
                    f.setUnderlineStyle(QTextCharFormat::SingleUnderline);
                    f.setUnderlineColor(col);
                    QColor tint = col;
                    tint.setAlphaF(sev <= 1 ? 0.16 : 0.08);
                    f.setBackground(tint);
                    setFormat(i, 1, f);
                }
            });
        }
    }

private:
    QQuickTextDocument* m_quick = nullptr;
    QString m_analysis;
    QJsonArray m_spans, m_errors;
    QVariantMap m_colors;
    QColor m_errorColor = QColor(0xe0, 0x5a, 0x5a);
    QColor m_warningColor = QColor(0xe5, 0xc0, 0x7b);
    QColor m_infoColor = QColor(0x61, 0xaf, 0xef);
};
