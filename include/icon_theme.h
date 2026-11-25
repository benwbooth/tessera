// The icon theme following the app's light or dark look: breeze on light,
// breeze-dark on dark (without Plasma's platform theme Qt picks plain
// breeze, dark grey icons that vanish on a dark menu). A theme of the
// desktop's own that isn't Breeze is left as it is. (grove's the same.)
#pragma once

#include <QtCore/QHash>
#include <QtCore/QObject>
#include <QtGui/QColor>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtCore/QUrl>
#include <QtCore/QUrlQuery>
#include <QtQml/QQmlEngine>
#include <QtQuick/QQuickImageProvider>

// Theme icons for menus, drawn on a loader thread (making a menu's rows
// looked each one up on the UI thread: ~0.3 s for the terminal's menu):
//     image://icon/<name>?px=<device px>&mono=<rrggbb>&r=<rev>
// At exactly px pixels (a design size of the theme, drawn 1:1). A
// one-colour dark icon (Breeze's monochrome ones, black without KDE's icon
// engine) is drawn in `mono`; colourful ones as they are. (From grove's.)
class TesseraIconProvider : public QQuickImageProvider {
public:
    // (QImage: pixmaps aren't safe off the UI thread.)
    TesseraIconProvider() : QQuickImageProvider(QQuickImageProvider::Image, QQuickImageProvider::ForceAsynchronousImageLoading) {}
    QImage requestImage(const QString& id, QSize* size, const QSize& req) override
    {
        QString key = id;
        int px = qMax(req.width(), req.height());
        QColor mono;
        const int q = key.lastIndexOf(QLatin1Char('?'));
        if (q >= 0) {
            const QUrlQuery query(key.mid(q + 1));
            if (query.hasQueryItem(QStringLiteral("px"))) px = query.queryItemValue(QStringLiteral("px")).toInt();
            if (query.hasQueryItem(QStringLiteral("mono"))) mono = QColor(QLatin1Char('#') + query.queryItemValue(QStringLiteral("mono")));
            key.truncate(q);
        }
        px = qBound(8, px, 256);
        QIcon icon;
        for (const QString& n : QUrl::fromPercentEncoding(key.toUtf8()).split(QLatin1Char('|'), Qt::SkipEmptyParts))
            if (QIcon::hasThemeIcon(n)) { icon = QIcon::fromTheme(n); break; }
        QImage img = icon.isNull() ? QImage() : icon.pixmap(QSize(px, px), 1.0).toImage();
        if (img.isNull()) { if (size) *size = QSize(); return img; }
        img = img.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        img.setDevicePixelRatio(1.0);
        if (mono.isValid() && IconTheme_isDarkMono(img)) {
            QPainter p(&img);
            p.setCompositionMode(QPainter::CompositionMode_SourceIn);
            p.fillRect(img.rect(), mono);
        }
        if (size) *size = img.size();
        return img;
    }

private:
    // One colour, dark grey or black (see IconTheme::mono).
    static bool IconTheme_isDarkMono(const QImage& src)
    {
        const QImage img = src.convertToFormat(QImage::Format_ARGB32);
        bool any = false;
        QRgb first = 0;
        for (int y = 0; y < img.height(); y++) {
            const QRgb* row = reinterpret_cast<const QRgb*>(img.constScanLine(y));
            for (int x = 0; x < img.width(); x++) {
                if (qAlpha(row[x]) < 96) continue;
                if (!any) { first = row[x]; any = true; continue; }
                if (qAbs(qRed(row[x]) - qRed(first)) > 24 || qAbs(qGreen(row[x]) - qGreen(first)) > 24 || qAbs(qBlue(row[x]) - qBlue(first)) > 24) return false;
            }
        }
        const QColor c = QColor::fromRgb(first);
        return any && c.hslSaturationF() < 0.25 && c.lightnessF() < 0.3;
    }
};

class IconTheme : public QObject {
    Q_OBJECT

public:
    explicit IconTheme(QObject* parent = nullptr) : QObject(parent) {}

    // Whether it changed (rows already drawn need their icons again).
    Q_INVOKABLE bool setDark(bool dark)
    {
        // (The menus' icon provider, on this engine: image://icon/...)
        if (QQmlEngine* e = qmlEngine(this); e && !e->imageProvider(QStringLiteral("icon")))
            e->addImageProvider(QStringLiteral("icon"), new TesseraIconProvider);
        static const QString system = QIcon::themeName();
        if (!system.isEmpty() && !system.startsWith(QLatin1String("breeze"), Qt::CaseInsensitive)) return false;
        const QString want = dark ? QStringLiteral("breeze-dark") : QStringLiteral("breeze");
        if (QIcon::themeName() == want) return false;
        QIcon::setThemeName(want);
        return true;
    }

    // Whether an icon is a single colour (Breeze's monochrome ones): those
    // are drawn in the text's colour. (Breeze colours them with CSS
    // currentColor, which only KDE's icon engine fills in; plain Qt draws
    // them black, whatever the theme.) Colourful ones are left as they are.
    Q_INVOKABLE bool mono(const QString& name)
    {
        if (name.isEmpty()) return false;
        const QString key = QIcon::themeName() + QLatin1Char('/') + name;
        auto it = m_mono.constFind(key);
        if (it != m_mono.constEnd()) return *it;
        const QImage img = QIcon::fromTheme(name).pixmap(22, 22).toImage().convertToFormat(QImage::Format_ARGB32);
        bool any = false, one = true;
        QRgb first = 0;
        for (int y = 0; y < img.height() && one; y++) {
            const QRgb* row = reinterpret_cast<const QRgb*>(img.constScanLine(y));
            for (int x = 0; x < img.width(); x++) {
                if (qAlpha(row[x]) < 96) continue;   // (edges blend with nothing)
                if (!any) { first = row[x]; any = true; continue; }
                if (qAbs(qRed(row[x]) - qRed(first)) > 24 || qAbs(qGreen(row[x]) - qGreen(first)) > 24 || qAbs(qBlue(row[x]) - qBlue(first)) > 24) { one = false; break; }
            }
        }
        // (And that colour the dark grey or black they come out as: a red
        // one, Quit's, is red on purpose.)
        const QColor c = QColor::fromRgb(first);
        const bool r = any && one && c.hslSaturationF() < 0.25 && c.lightnessF() < 0.3;
        m_mono.insert(key, r);
        return r;
    }

private:
    QHash<QString, bool> m_mono;
};
