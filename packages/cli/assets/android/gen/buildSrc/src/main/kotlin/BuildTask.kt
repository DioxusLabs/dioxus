import java.io.File
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.logging.LogLevel
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.TaskAction

open class BuildTask : DefaultTask() {
    @Input
    var rootDirRel: String? = null
    @Input
    var target: String? = null
    @Input
    var release: Boolean? = null

    @TaskAction
    fun build() {
        // Normally cargo would be a sub-child of the project, but `dx` is, so we comment out the build stuff
        // We will eventually be passing in the binary manually
        //
        // val rootDirRel = rootDirRel ?: throw GradleException("rootDirRel cannot be null")
        // val target = target ?: throw GradleException("target cannot be null")
        // val release = release ?: throw GradleException("release cannot be null")

        // project.exec {
        //     workingDir(File(project.projectDir, rootDirRel))
        //     executable("cargo")
        //     args(listOf("android", "build"))
        //     if (project.logger.isEnabled(LogLevel.DEBUG)) {
        //         args("-vv")
        //     } else if (project.logger.isEnabled(LogLevel.INFO)) {
        //         args("-v")
        //     }
        //     if (release) {
        //         args("--release")
        //     }
        //     args(target)
        // }.assertNormalExitValue()
    }
}

