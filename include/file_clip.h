// Files on the desktop's clipboard, as file managers put them there and take
// them: their URLs (text/uri-list), GNOME's and KDE's copy-or-cut marks, and
// the paths as text. The folder view's Copy, Cut and Paste use it, so files
// go to and from Dolphin, Nautilus and the rest.
#pragma once

#include <QtCore/QMimeData>
#include <QtCore/QObject>
#include <QtCore/QUrl>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>

class FileClip : public QObject {
    Q_OBJECT
    // Whether the clipboard has files (local ones), and whether they were cut.
    Q_PROPERTY(bool hasFiles READ hasFiles NOTIFY changed)
    Q_PROPERTY(bool cut READ cut NOTIFY changed)

public:
    explicit FileClip(QObject* parent = nullptr) : QObject(parent)
    {
        connect(QGuiApplication::clipboard(), &QClipboard::dataChanged, this, &FileClip::changed);
    }

    Q_INVOKABLE void put(const QStringList& paths, bool cut)
    {
        auto* d = new QMimeData;
        QList<QUrl> urls;
        QByteArray gnome = cut ? "cut" : "copy";
        for (const QString& p : paths) {
            const QUrl u = QUrl::fromLocalFile(p);
            urls << u;
            gnome += "\n" + u.toEncoded();
        }
        d->setUrls(urls);
        d->setData(QStringLiteral("x-special/gnome-copied-files"), gnome);
        d->setData(QStringLiteral("application/x-kde-cutselection"), cut ? "1" : "0");
        d->setText(paths.join(QLatin1Char('\n')));
        QGuiApplication::clipboard()->setMimeData(d);
    }
    Q_INVOKABLE QStringList files() const
    {
        QStringList out;
        if (const QMimeData* d = QGuiApplication::clipboard()->mimeData())
            for (const QUrl& u : d->urls())
                if (u.isLocalFile()) out << u.toLocalFile();
        return out;
    }
    bool hasFiles() const { return !files().isEmpty(); }
    bool cut() const
    {
        const QMimeData* d = QGuiApplication::clipboard()->mimeData();
        if (!d) return false;
        if (d->hasFormat(QStringLiteral("application/x-kde-cutselection"))) return d->data(QStringLiteral("application/x-kde-cutselection")).startsWith('1');
        return d->data(QStringLiteral("x-special/gnome-copied-files")).startsWith("cut");
    }
    // (Cut files pasted: the clipboard doesn't keep offering them.)
    Q_INVOKABLE void clear() { QGuiApplication::clipboard()->clear(); }

signals:
    void changed();
};
