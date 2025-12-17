module.exports = [
"[project]/web/lib/supabase/server.ts [app-rsc] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "createClient",
    ()=>createClient
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f40$supabase$2f$ssr$2f$dist$2f$module$2f$index$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/node_modules/@supabase/ssr/dist/module/index.js [app-rsc] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f40$supabase$2f$ssr$2f$dist$2f$module$2f$createServerClient$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/node_modules/@supabase/ssr/dist/module/createServerClient.js [app-rsc] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$headers$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/node_modules/next/headers.js [app-rsc] (ecmascript)");
;
;
async function createClient() {
    const cookieStore = await (0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$headers$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["cookies"])();
    return (0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f40$supabase$2f$ssr$2f$dist$2f$module$2f$createServerClient$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["createServerClient"])(("TURBOPACK compile-time value", "http://127.0.0.1:54321"), ("TURBOPACK compile-time value", "sb_publishable_ACJWlzQHlZjBrEguHvfOxg_3BJgxAaH"), {
        cookies: {
            getAll () {
                return cookieStore.getAll();
            },
            setAll (cookiesToSet) {
                try {
                    cookiesToSet.forEach(({ name, value, options })=>cookieStore.set(name, value, options));
                } catch  {
                // The `setAll` method was called from a Server Component.
                // This can be ignored if you have proxy refreshing
                // user sessions.
                }
            }
        }
    });
}
}),
"[project]/web/lib/utils.ts [app-rsc] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "cn",
    ()=>cn,
    "getURL",
    ()=>getURL,
    "hasEnvVars",
    ()=>hasEnvVars
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$clsx$2f$dist$2f$clsx$2e$mjs__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/node_modules/clsx/dist/clsx.mjs [app-rsc] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$tailwind$2d$merge$2f$dist$2f$bundle$2d$mjs$2e$mjs__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/node_modules/tailwind-merge/dist/bundle-mjs.mjs [app-rsc] (ecmascript)");
;
;
function cn(...inputs) {
    return (0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$tailwind$2d$merge$2f$dist$2f$bundle$2d$mjs$2e$mjs__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["twMerge"])((0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$clsx$2f$dist$2f$clsx$2e$mjs__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["clsx"])(inputs));
}
const hasEnvVars = ("TURBOPACK compile-time value", "http://127.0.0.1:54321") && ("TURBOPACK compile-time value", "sb_publishable_ACJWlzQHlZjBrEguHvfOxg_3BJgxAaH");
function getURL() {
    let url = process?.env?.NEXT_PUBLIC_SITE_URL ?? // Set this to your site URL in production env.
    process?.env?.NEXT_PUBLIC_VERCEL_URL ?? // Automatically set by Vercel.
    "http://127.0.0.1:3000/";
    // Make sure to include `https://` when not localhost.
    url = url.startsWith("http") ? url : `https://${url}`;
    // Make sure to include a trailing `/`.
    url = url.endsWith("/") ? url : `${url}/`;
    return url;
}
}),
"[project]/web/auth/auth-actions.ts [app-rsc] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/* __next_internal_action_entry_do_not_use__ [{"406da1a8720add738f7a516daa683fd97fc0dfdbb9":"signInWithPassword","407a313724edc8c6fb240fe552e20005a12a1cb3ee":"signInWithEmailOtp","408bb15ca5145283feb32e81e6b7553c7b0351a6d4":"signUp","40c1a120f595d78ea9b02e95d6f99f9d9d40b3ade6":"resetPasswordForEmail","40ec4cc7b127b8d8d8c755b99a8d097fa925cfd280":"updateUserPassword"},"",""] */ __turbopack_context__.s([
    "resetPasswordForEmail",
    ()=>resetPasswordForEmail,
    "signInWithEmailOtp",
    ()=>signInWithEmailOtp,
    "signInWithPassword",
    ()=>signInWithPassword,
    "signUp",
    ()=>signUp,
    "updateUserPassword",
    ()=>updateUserPassword
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$server$2d$reference$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/node_modules/next/dist/build/webpack/loaders/next-flight-loader/server-reference.js [app-rsc] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$supabase$2f$server$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/web/lib/supabase/server.ts [app-rsc] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$utils$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/web/lib/utils.ts [app-rsc] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$cache$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/node_modules/next/cache.js [app-rsc] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$action$2d$validate$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/node_modules/next/dist/build/webpack/loaders/next-flight-loader/action-validate.js [app-rsc] (ecmascript)");
;
;
;
;
async function signUp(formData) {
    const email = formData.get("email");
    const password = formData.get("password");
    const repeatPassword = formData.get("repeatPassword");
    console.log("[signUp] Attempting sign up for email:", email);
    if (!email || !password) {
        console.log("[signUp] Validation failed: missing email or password");
        return {
            error: "Email and password are required"
        };
    }
    if (password !== repeatPassword) {
        console.log("[signUp] Validation failed: passwords do not match");
        return {
            error: "Passwords do not match"
        };
    }
    const supabase = await (0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$supabase$2f$server$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["createClient"])();
    const redirectUrl = `${(0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$utils$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["getURL"])()}protected`;
    console.log("[signUp] Redirect URL:", redirectUrl);
    const { error } = await supabase.auth.signUp({
        email,
        password,
        options: {
            emailRedirectTo: redirectUrl
        }
    });
    if (error) {
        console.error("[signUp] Error:", error.message);
        return {
            error: error.message
        };
    }
    console.log("[signUp] Success: user signed up");
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$cache$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["revalidatePath"])("/", "layout");
    return {
        success: true,
        redirectTo: "/auth/sign-up-success"
    };
}
async function signInWithPassword(formData) {
    const email = formData.get("email");
    const password = formData.get("password");
    console.log("[signInWithPassword] Attempting sign in for email:", email);
    if (!email || !password) {
        console.log("[signInWithPassword] Validation failed: missing email or password");
        return {
            error: "Email and password are required"
        };
    }
    const supabase = await (0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$supabase$2f$server$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["createClient"])();
    const { error } = await supabase.auth.signInWithPassword({
        email,
        password
    });
    if (error) {
        console.error("[signInWithPassword] Error:", error.message);
        return {
            error: error.message
        };
    }
    console.log("[signInWithPassword] Success: user signed in");
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$cache$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["revalidatePath"])("/", "layout");
    return {
        success: true,
        redirectTo: "/protected"
    };
}
async function signInWithEmailOtp(formData) {
    const email = formData.get("email");
    console.log("[signInWithEmailOtp] Attempting magic link sign in for email:", email);
    if (!email) {
        console.log("[signInWithEmailOtp] Validation failed: email is required");
        return {
            error: "Email is required"
        };
    }
    const supabase = await (0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$supabase$2f$server$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["createClient"])();
    const redirectUrl = `${(0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$utils$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["getURL"])()}protected`;
    console.log("[signInWithEmailOtp] Redirect URL after verification:", redirectUrl);
    const { error } = await supabase.auth.signInWithOtp({
        email,
        options: {
            emailRedirectTo: redirectUrl
        }
    });
    if (error) {
        console.error("[signInWithEmailOtp] Error:", error.message);
        return {
            error: error.message
        };
    }
    console.log("[signInWithEmailOtp] Success:  email sent");
    return {
        success: true
    };
}
async function resetPasswordForEmail(formData) {
    const email = formData.get("email");
    console.log("[resetPasswordForEmail] Requesting password reset for email:", email);
    if (!email) {
        console.log("[resetPasswordForEmail] Validation failed: email is required");
        return {
            error: "Email is required"
        };
    }
    const supabase = await (0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$supabase$2f$server$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["createClient"])();
    const redirectUrl = `${(0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$utils$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["getURL"])()}auth/update-password`;
    console.log("[resetPasswordForEmail] Redirect URL:", redirectUrl);
    const { error } = await supabase.auth.resetPasswordForEmail(email, {
        redirectTo: redirectUrl
    });
    if (error) {
        console.error("[resetPasswordForEmail] Error:", error.message);
        return {
            error: error.message
        };
    }
    console.log("[resetPasswordForEmail] Success: password reset email sent");
    return {
        success: true
    };
}
async function updateUserPassword(formData) {
    const password = formData.get("password");
    console.log("[updateUserPassword] Attempting to update password");
    if (!password) {
        console.log("[updateUserPassword] Validation failed: password is required");
        return {
            error: "Password is required"
        };
    }
    const supabase = await (0, __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$lib$2f$supabase$2f$server$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["createClient"])();
    const { error } = await supabase.auth.updateUser({
        password
    });
    if (error) {
        console.error("[updateUserPassword] Error:", error.message);
        return {
            error: error.message
        };
    }
    console.log("[updateUserPassword] Success: password updated");
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$cache$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["revalidatePath"])("/", "layout");
    return {
        success: true,
        redirectTo: "/protected"
    };
}
;
(0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$action$2d$validate$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["ensureServerEntryExports"])([
    signUp,
    signInWithPassword,
    signInWithEmailOtp,
    resetPasswordForEmail,
    updateUserPassword
]);
(0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$server$2d$reference$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["registerServerReference"])(signUp, "408bb15ca5145283feb32e81e6b7553c7b0351a6d4", null);
(0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$server$2d$reference$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["registerServerReference"])(signInWithPassword, "406da1a8720add738f7a516daa683fd97fc0dfdbb9", null);
(0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$server$2d$reference$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["registerServerReference"])(signInWithEmailOtp, "407a313724edc8c6fb240fe552e20005a12a1cb3ee", null);
(0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$server$2d$reference$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["registerServerReference"])(resetPasswordForEmail, "40c1a120f595d78ea9b02e95d6f99f9d9d40b3ade6", null);
(0, __TURBOPACK__imported__module__$5b$project$5d2f$node_modules$2f$next$2f$dist$2f$build$2f$webpack$2f$loaders$2f$next$2d$flight$2d$loader$2f$server$2d$reference$2e$js__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["registerServerReference"])(updateUserPassword, "40ec4cc7b127b8d8d8c755b99a8d097fa925cfd280", null);
}),
"[project]/web/.next-internal/server/app/auth/login/page/actions.js { ACTIONS_MODULE0 => \"[project]/web/auth/auth-actions.ts [app-rsc] (ecmascript)\" } [app-rsc] (server actions loader, ecmascript) <locals>", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([]);
var __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$auth$2f$auth$2d$actions$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/web/auth/auth-actions.ts [app-rsc] (ecmascript)");
;
;
}),
"[project]/web/.next-internal/server/app/auth/login/page/actions.js { ACTIONS_MODULE0 => \"[project]/web/auth/auth-actions.ts [app-rsc] (ecmascript)\" } [app-rsc] (server actions loader, ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "406da1a8720add738f7a516daa683fd97fc0dfdbb9",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$web$2f$auth$2f$auth$2d$actions$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["signInWithPassword"],
    "407a313724edc8c6fb240fe552e20005a12a1cb3ee",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$web$2f$auth$2f$auth$2d$actions$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__["signInWithEmailOtp"]
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$web$2f2e$next$2d$internal$2f$server$2f$app$2f$auth$2f$login$2f$page$2f$actions$2e$js__$7b$__ACTIONS_MODULE0__$3d3e$__$225b$project$5d2f$web$2f$auth$2f$auth$2d$actions$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$2922$__$7d$__$5b$app$2d$rsc$5d$__$28$server__actions__loader$2c$__ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i('[project]/web/.next-internal/server/app/auth/login/page/actions.js { ACTIONS_MODULE0 => "[project]/web/auth/auth-actions.ts [app-rsc] (ecmascript)" } [app-rsc] (server actions loader, ecmascript) <locals>');
var __TURBOPACK__imported__module__$5b$project$5d2f$web$2f$auth$2f$auth$2d$actions$2e$ts__$5b$app$2d$rsc$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/web/auth/auth-actions.ts [app-rsc] (ecmascript)");
}),
];

//# sourceMappingURL=web_2171197e._.js.map