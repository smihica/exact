// Video and audio elements' players and an image's picture in picture (LLP
// 1047.001 D10, the roster's `video`): the video arm's Swift half (LLP 1098;
// LLP 1042 §9). An app's composition links this module when its plan has a
// `video` or `audio` element or asks for picture in picture, and installs it
// before any session is made; the core reaches it only through `MediaLink`.
import ExactKit
import ExactHeavyLeaves

public enum ExactVideo {
    public static func install() {
        ExactHeavyLeaves.install()
        MediaLink.makeVideo = { VideoView(owner: $0) }
        #if os(iOS) || os(tvOS)
        MediaLink.claimsAudioSession()
        #endif
        MediaLink.pictureCommand = { presenter, name, args in
            if name == "requestPictureInPicture" { presenter.pictureInPicture(args) } else { PictureInPictureHost.exit(presenter.session) }
        }
        MediaLink.pictureChanged = { PictureInPictureHost.changed($0) }
        MediaLink.pictureGone = { PictureInPictureHost.gone($0) }
        #if os(macOS) && EXACT_AGENT
        MediaLink.pictureDecorate = { $0.decoratePictureInPicture($1) }
        MediaLink.pictureTap = { $0.pictureInPictureTap($1, control: $2) }
        MediaLink.pictureShot = { $0.pictureInPictureShot($1, of: $2) }
        #endif
    }
}
